# インストールと常駐

PocketRepo を開発マシンでローカルにビルドし、常時起動デーモンとして動かして
Tailscale 経由でスマホから使うための手順です（Homebrew への公開は不要）。

## 1. 設定ファイル

既定の設定ファイルは `~/.config/pocket-repo/config.toml` です。

```toml
bind = "0.0.0.0"        # 省略時 0.0.0.0
port = 7376             # 省略時 3000（7376 = 電話キーパッドで REPO）

# ghq 配下の git リポジトリを自動発見
scan_roots = ["~/ghq"]

# 個別指定・表示名の上書きも可能
# [[repos]]
# path = "~/work/some-repo"
# name = "work-repo"
```

CLI 引数でリポジトリを追加したり、設定を上書きすることもできます:

```sh
pocket-repo ~/path/to/repo-a --port 8080
pocket-repo --help
```

## 2. バイナリをインストール

```sh
cargo install --path .     # -> ~/.cargo/bin/pocket-repo
```

## 3. 常駐化（launchd = macOS の systemd 相当）

`packaging/launchd/com.ppdx999.pocket-repo.plist` をプレースホルダ展開して
`~/Library/LaunchAgents/` に置き、読み込みます。

```sh
mkdir -p ~/Library/LaunchAgents ~/Library/Logs
sed "s#__BIN__#$HOME/.cargo/bin/pocket-repo#; s#__HOME__#$HOME#" \
  packaging/launchd/com.ppdx999.pocket-repo.plist \
  > ~/Library/LaunchAgents/com.ppdx999.pocket-repo.plist

launchctl load ~/Library/LaunchAgents/com.ppdx999.pocket-repo.plist
```

- ログイン時に自動起動し、落ちたら自動再起動します（`RunAtLoad` + `KeepAlive`）。
- 引数なしで起動し `~/.config/pocket-repo/config.toml` を読みます。
- ログ: `~/Library/Logs/pocket-repo.log`

管理コマンド:

```sh
launchctl unload ~/Library/LaunchAgents/com.ppdx999.pocket-repo.plist   # 停止
launchctl load   ~/Library/LaunchAgents/com.ppdx999.pocket-repo.plist   # 起動
launchctl kickstart -k gui/$(id -u)/com.ppdx999.pocket-repo             # 再起動
# 設定を変えたら kickstart で再読み込み。バイナリ更新は cargo install 後に kickstart。
```

## 4. Tailscale 経由でアクセス

Tailnet 内のスマホから、開発マシンの Tailscale IP で開きます:

```
http://<開発マシンの100.x.x.x>:7376
```

Tailscale IP は `tailscale ip -4` で確認できます。

> セキュリティ: 認証は無く Tailnet がアクセス境界です。より締めるなら
> `bind = "<Tailscale IP>"`（Tailnet のみ）か、`bind = "127.0.0.1"` +
> `tailscale serve` を検討してください。

---

## 補足: Homebrew formula について

`packaging/homebrew/pocket-repo.rb` に `brew services` 用の formula も置いてあります
が、これは将来 Homebrew（tap）で配布したくなった場合向けです。ローカル常駐だけなら
上記の launchd 手順で完結します。
