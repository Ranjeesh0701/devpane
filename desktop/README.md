# Devpane Desktop

Native Tauri v2 standalone desktop application with an embedded zero-click background daemon for real-time process monitoring and live log viewing.

## macOS Gatekeeper / Unsigned DMG Note

If macOS Gatekeeper flags the downloaded `.dmg` as *"damaged and can't be opened"*, copy `devpane.app` to `/Applications` and run:

```bash
xattr -cr /Applications/devpane.app && codesign --force --deep --sign - /Applications/devpane.app
```

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
