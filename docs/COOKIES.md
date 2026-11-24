# Manual cookies

Some providers use a Cookie header for web usage. Open **Settings > Providers**, select the provider, and paste a Cookie header in the **Browser Cookies** section. **Save** stores the header and converts the legacy **Automatic** source to **Manual**. Use **Remove** to clear it or select **Disabled** where available.

The desktop app stores manual cookies in `manual_cookies.json` under the CodexBar config directory, protected by Windows DPAPI. Earlier versions also stored imported headers in this file. Saved headers remain usable after upgrading. Automatic browser and desktop session discovery has been removed.

The CLI accepts a header with `codexbar config set-cookie`. See [CLI](CLI.md) for options.
