//! Kimi web usage with a user-supplied Cookie header.

use reqwest::Client;

use super::code_api::MISSING_WEEKLY_DESCRIPTION;
use super::credential::KimiWebCredential;
use super::{
    KIMI_SUBSCRIPTION_SERVICE, KIMI_SUBSCRIPTION_STATS_SERVICE, KIMI_WEB_USAGE_SERVICE,
    KimiProvider, KimiRegion, KimiSubscriptionResponse, KimiSubscriptionStatsResponse,
    KimiWebUsageResponse, apply_subscription_windows, gift_display_details, kimi_web_post,
};
use crate::core::{ProviderError, ProviderFetchResult, RateWindow, UsageSnapshot};

/// A failed web fetch. `had_token` records whether web auth had a token to
/// send (upstream `KimiWebFetchStrategy.isAvailable`); Auto mode reports an
/// earlier CLI failure only when web auth had nothing to try.
#[derive(Debug)]
pub(super) struct WebFetchFailure {
    pub(super) error: ProviderError,
    pub(super) had_token: bool,
}

impl WebFetchFailure {
    fn before_token(error: ProviderError) -> Self {
        Self {
            error,
            had_token: false,
        }
    }

    fn after_token(error: ProviderError) -> Self {
        Self {
            error,
            had_token: true,
        }
    }
}

/// Read the explicitly configured web credential for optional plan details.
pub(super) fn web_auth_credentials(
    manual_header: Option<&str>,
    _region: KimiRegion,
) -> Vec<KimiWebCredential> {
    selected_account_auth_token(manual_header)
        .map(KimiWebCredential::plain)
        .into_iter()
        .collect()
}

pub(crate) async fn fetch_via_web(
    cookie_header: Option<&str>,
    region: KimiRegion,
    account_isolated: bool,
) -> Result<ProviderFetchResult, ProviderError> {
    fetch_web_session_isolated(cookie_header, region, account_isolated)
        .await
        .map_err(|failure| failure.error)
}

/// [`fetch_via_web`] entry point. A selected token account is an identity
/// boundary: fetch only its own session cookie and never fall back to another
/// ambient credential or browser account (upstream per-provider token account
/// routing).
pub(super) async fn fetch_web_session_isolated(
    cookie_header: Option<&str>,
    region: KimiRegion,
    _account_isolated: bool,
) -> Result<ProviderFetchResult, WebFetchFailure> {
    fetch_web_session(cookie_header, region).await
}

/// [`fetch_via_web`], also reporting whether web auth had a token to try.
pub(super) async fn fetch_web_session(
    cookie_header: Option<&str>,
    region: KimiRegion,
) -> Result<ProviderFetchResult, WebFetchFailure> {
    let token =
        selected_account_auth_token(cookie_header).map_err(WebFetchFailure::before_token)?;
    let http = client().map_err(WebFetchFailure::before_token)?;
    fetch_via_web_token(&http, &KimiWebCredential::plain(token), region)
        .await
        .map_err(WebFetchFailure::after_token)
}

fn selected_account_auth_token(cookie_header: Option<&str>) -> Result<String, ProviderError> {
    cookie_header
        .and_then(|header| KimiProvider::auth_token_from_cookie_header(header).ok())
        .ok_or(ProviderError::AuthRequired)
}

pub(super) fn client() -> Result<reqwest::Client, ProviderError> {
    crate::core::credentialed_http_client_builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ProviderError::Other(e.to_string()))
}

async fn fetch_via_web_token(
    client: &reqwest::Client,
    credential: &KimiWebCredential,
    region: KimiRegion,
) -> Result<ProviderFetchResult, ProviderError> {
    let usage_url = region.web_api_url(KIMI_WEB_USAGE_SERVICE);
    let resp = kimi_web_post(
        client,
        &usage_url,
        region,
        credential,
        serde_json::json!({ "scope": ["FEATURE_CODING"] }),
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(ProviderError::AuthRequired);
        }
        return Err(ProviderError::Other(format!("API error: {}", status)));
    }

    let usage: KimiWebUsageResponse = resp
        .json()
        .await
        .map_err(|e| ProviderError::Parse(e.to_string()))?;

    let (subscription, plan_name) = fetch_subscription_details(client, credential, region).await;

    snapshot_from_web_usage_response_with_plan(usage, subscription, plan_name)
}

const SUBSCRIPTION_ENRICHMENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

async fn fetch_subscription_details(
    client: &reqwest::Client,
    credential: &KimiWebCredential,
    region: KimiRegion,
) -> (Option<KimiSubscriptionStatsResponse>, Option<String>) {
    // The quota statistics and the optional title are independent. Keep a
    // completed statistics response when the plan endpoint is slow or absent.
    let stats = tokio::time::timeout(
        SUBSCRIPTION_ENRICHMENT_TIMEOUT,
        fetch_subscription_for_enrichment_result(client, credential, region),
    );
    let plan = tokio::time::timeout(
        SUBSCRIPTION_ENRICHMENT_TIMEOUT,
        fetch_subscription_plan(client, credential, region),
    );
    let (stats, plan) = tokio::join!(stats, plan);
    (
        stats.ok().and_then(Result::ok).flatten(),
        plan.ok().flatten(),
    )
}

pub(super) fn snapshot_from_web_usage_response(
    response: KimiWebUsageResponse,
    subscription: Option<KimiSubscriptionStatsResponse>,
) -> Result<UsageSnapshot, ProviderError> {
    snapshot_from_web_usage_response_with_plan(response, subscription, None)
        .map(|fetched| fetched.usage)
}

fn snapshot_from_web_usage_response_with_plan(
    response: KimiWebUsageResponse,
    subscription: Option<KimiSubscriptionStatsResponse>,
    plan_name: Option<String>,
) -> Result<ProviderFetchResult, ProviderError> {
    let coding = response
        .usages
        .unwrap_or_default()
        .into_iter()
        .find(|usage| usage.scope == "FEATURE_CODING");
    let has_coding = coding.is_some();
    let mut usage = match coding {
        Some(coding) => {
            let primary = KimiProvider::rate_window_from_usage_detail(&coding.detail, Some(10080))?;
            let mut usage = UsageSnapshot::new(primary).with_login_method("Kimi");

            if let Some(limit) = coding.limits.unwrap_or_default().into_iter().next() {
                let window_minutes = limit.window.as_ref().and_then(super::kimi_window_minutes);
                let rate_limit =
                    KimiProvider::rate_window_from_usage_detail(&limit.detail, window_minutes)?;
                usage = usage.with_secondary(rate_limit);
            }
            usage
        }
        // Upstream #4700: a lapsed plan can omit its Code pools while the
        // membership pools below still show usage.
        None => UsageSnapshot::new(RateWindow::informational(MISSING_WEEKLY_DESCRIPTION))
            .with_login_method("Kimi"),
    };

    let mut details = Vec::new();
    if let Some(subscription) = subscription.as_ref() {
        details = super::monthly_breakdown::monthly_breakdown_details(
            super::monthly_breakdown::subscription_monthly_shares(subscription),
        );
        details.extend(gift_display_details(subscription, usage.updated_at));
        usage = apply_subscription_windows(usage, subscription);
    }
    if let Some(plan_name) = plan_name {
        usage = usage.with_login_method(plan_name);
    }

    // Without Code pools, the membership pools must supply at least one lane.
    if !has_coding && usage.extra_rate_windows.is_empty() {
        return Err(ProviderError::Parse(
            "FEATURE_CODING scope not found in response".into(),
        ));
    }

    Ok(ProviderFetchResult::new(usage, "web").with_display_details(details))
}

pub(super) async fn fetch_subscription_plan(
    client: &Client,
    credential: &KimiWebCredential,
    region: KimiRegion,
) -> Option<String> {
    let url = region.web_api_url(KIMI_SUBSCRIPTION_SERVICE);
    match kimi_web_post(client, &url, region, credential, serde_json::json!({})).await {
        Ok(response) if response.status().is_success() => response
            .json::<KimiSubscriptionResponse>()
            .await
            .ok()
            .and_then(|response| response.plan_name()),
        _ => None,
    }
}

pub(super) async fn fetch_subscription_for_enrichment_result(
    client: &Client,
    credential: &KimiWebCredential,
    region: KimiRegion,
) -> Result<Option<KimiSubscriptionStatsResponse>, ProviderError> {
    let url = region.web_api_url(KIMI_SUBSCRIPTION_STATS_SERVICE);
    match kimi_web_post(client, &url, region, credential, serde_json::json!({})).await {
        Ok(response) if response.status().is_success() => response
            .json()
            .await
            .map(Some)
            .map_err(|error| ProviderError::Parse(error.to_string())),
        Ok(response) if response.status().as_u16() == 401 || response.status().as_u16() == 403 => {
            Err(ProviderError::AuthRequired)
        }
        Ok(_) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_session_rejects_missing_or_invalid_cookie_without_fallback() {
        assert!(matches!(
            selected_account_auth_token(None),
            Err(ProviderError::AuthRequired)
        ));
        assert!(matches!(
            selected_account_auth_token(Some("locale=en-US")),
            Err(ProviderError::AuthRequired)
        ));
        assert_eq!(
            selected_account_auth_token(Some("Cookie: kimi-auth=selected")).unwrap(),
            "selected"
        );
    }

    #[test]
    fn subscription_stats_do_not_invent_a_membership_label() {
        let usage: KimiWebUsageResponse = serde_json::from_value(serde_json::json!({
            "usages": [{
                "scope": "FEATURE_CODING",
                "detail": { "limit": "1000", "used": "125" }
            }]
        }))
        .unwrap();
        let subscription: KimiSubscriptionStatsResponse =
            serde_json::from_value(serde_json::json!({
                "subscriptionBalance": {
                    "amountUsedRatio": 0.25,
                    "expireTime": "2026-09-30T00:00:00Z"
                },
                "ratelimitCode7d": {
                    "ratio": 0.1,
                    "enabled": true,
                    "resetTime": "2026-09-14T00:00:00Z"
                }
            }))
            .unwrap();

        let snapshot = snapshot_from_web_usage_response(usage, Some(subscription)).unwrap();

        assert_eq!(snapshot.login_method.as_deref(), Some("Kimi"));
        assert!(
            snapshot
                .extra_rate_windows
                .iter()
                .any(|window| window.id == "kimi-monthly")
        );
    }

    #[test]
    fn active_subscription_title_is_used_but_inactive_title_is_ignored() {
        let active: KimiSubscriptionResponse = serde_json::from_value(serde_json::json!({
            "subscription": {
                "active": true,
                "status": "SUBSCRIPTION_STATUS_ACTIVE",
                "goods": { "title": "  Allegro  " }
            }
        }))
        .unwrap();
        assert_eq!(active.plan_name().as_deref(), Some("Allegro"));

        let inactive: KimiSubscriptionResponse = serde_json::from_value(serde_json::json!({
            "subscription": {
                "active": false,
                "status": "SUBSCRIPTION_STATUS_EXPIRED",
                "goods": { "title": "Allegro" }
            }
        }))
        .unwrap();
        assert_eq!(inactive.plan_name(), None);
    }

    #[test]
    fn lapsed_web_result_carries_gift_rows_and_membership_pool() {
        // Upstream #4700 + #4691: a lapsed plan keeps the membership pool and
        // the live gift rows in the web result, with the web source label.
        let usage: KimiWebUsageResponse = serde_json::from_value(serde_json::json!({
            "totalQuota": { "limit": "100", "used": "36" }
        }))
        .unwrap();
        let subscription: KimiSubscriptionStatsResponse =
            serde_json::from_value(serde_json::json!({
                "subscriptionBalance": {
                    "feature": "FEATURE_OMNI",
                    "type": "SUBSCRIPTION",
                    "amountUsedRatio": 0.36,
                    "expireTime": "2026-11-22T00:00:00Z"
                },
                "giftBalances": [{
                    "feature": "FEATURE_OMNI",
                    "type": "GIFT",
                    "amountUsedRatio": 0.6,
                    "expireTime": "2099-12-31T15:59:59Z",
                    "displayName": "Welcome Credit"
                }]
            }))
            .unwrap();

        let fetched =
            snapshot_from_web_usage_response_with_plan(usage, Some(subscription), None).unwrap();

        assert_eq!(fetched.source_label, "web");
        assert!(fetched.usage.primary.is_informational);
        assert!(
            fetched
                .usage
                .extra_rate_windows
                .iter()
                .any(|window| window.id == "kimi-monthly")
        );
        assert_eq!(fetched.display_details.len(), 1);
        assert_eq!(fetched.display_details[0].title(), "Welcome Credit");
        assert_eq!(fetched.display_details[0].value(), "40% left");
    }

    #[test]
    fn web_result_lists_the_total_usage_breakdown_before_gifts() {
        // Upstream #4759: the membership pool's Code share splits Total usage.
        let usage: KimiWebUsageResponse = serde_json::from_value(serde_json::json!({
            "totalQuota": { "limit": "100", "used": "36" }
        }))
        .unwrap();
        let subscription: KimiSubscriptionStatsResponse =
            serde_json::from_value(serde_json::json!({
                "subscriptionBalance": {
                    "feature": "FEATURE_OMNI",
                    "type": "SUBSCRIPTION",
                    "amountUsedRatio": 0.36,
                    "kimiCodeUsedRatio": 0.1,
                    "expireTime": "2026-11-22T00:00:00Z"
                },
                "giftBalances": [{
                    "feature": "FEATURE_OMNI",
                    "type": "GIFT",
                    "amountUsedRatio": 0.6,
                    "expireTime": "2099-12-31T15:59:59Z",
                    "displayName": "Welcome Credit"
                }]
            }))
            .unwrap();

        let fetched =
            snapshot_from_web_usage_response_with_plan(usage, Some(subscription), None).unwrap();

        let rows: Vec<(&str, &str)> = fetched
            .display_details
            .iter()
            .map(|row| (row.title(), row.value()))
            .collect();
        assert_eq!(
            rows,
            [
                ("Kimi Code", "10% used"),
                ("Other features", "26% used"),
                ("Welcome Credit", "40% left"),
            ]
        );
    }
}
