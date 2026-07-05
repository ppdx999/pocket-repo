# PocketRepo

PocketRepoは、**スマートフォンからGitリポジトリを快適に閲覧・探索するための、モバイルファーストなWebアプリケーション**です。

開発者は外出先で「この実装どうなっていたっけ？」「レビューコメントを返したい」「Claude Codeにこのファイルを読ませたい」と思うことがよくあります。
しかし、GitHubのWeb UIや一般的なファイルブラウザはスマートフォンでのコードリーディングに最適化されているとは言えません。

PocketRepoは、自宅や開発マシン上で動作する軽量なサーバー（単一バイナリ）として動き、Tailscaleなどのプライベートネットワーク経由でスマートフォンから安全にアクセスします。

## スクリーンショット

<p align="center">
  <img src="docs/screenshot-tree.jpeg" width="30%" alt="ディレクトリツリー">
  &nbsp;
  <img src="docs/screenshot-file.jpeg" width="30%" alt="ファイル閲覧（シンタックスハイライト）">
  &nbsp;
  <img src="docs/screenshot-diff.jpeg" width="30%" alt="Git diff（タイムライン・折りたたみ）">
</p>

<p align="center">
  <em>左から: ディレクトリツリー / ファイル閲覧 / Git diff</em>
</p>

## 目標

* スマホでコードを読むことに最適化されたUI
* Gitリポジトリを高速に閲覧できること
* 専用アプリ不要でブラウザから利用できること
* 単一バイナリで簡単に導入できること
* プライベートネットワーク内で安全に利用できること

## 主な機能

* **複数リポジトリ管理** — 設定ファイルの `scan_roots` で ghq 配下などを自動発見、または個別指定
* **ディレクトリツリー** — フォルダアイコンのタップでその場で展開／折りたたみ、開閉状態はブラウザに記憶（deep link での遷移も可）
* **ファイル閲覧＋シンタックスハイライト** — サーバー側（syntect）でハイライト
* **ファジーファイル検索** — どの画面からでもアイコンで起動（nucleo による fzf 風スコアリング）
* **Git diff** — 未コミットの作業ツリー → HEAD → 過去へと遡るタイムライン表示、ファイル単位で開閉
* **ブランチ／タグ選択** — 任意の ref を選んでツリー・ファイル・検索を閲覧（`?ref=` で共有可能）
* **最近閲覧したファイル** — 端末ごとに記憶（localStorage）
* **パスのコピー** — 各エントリからリポジトリルート相対パスをワンタップでコピー
* **レスポンス圧縮** — brotli / gzip / zstd に対応し、モバイル回線でも軽快
* **常駐化** — launchd（macOS）で常時起動・自動復帰

## 使い方

### ビルドとインストール

```sh
cargo install --path .   # -> ~/.cargo/bin/pocket-repo
```

### 設定

既定の設定ファイルは `~/.config/pocket-repo/config.toml`：

```toml
bind = "0.0.0.0"        # 省略時 0.0.0.0
port = 3000             # 省略時 3000

# ghq 配下の git リポジトリを自動発見
scan_roots = ["~/ghq"]

# 個別指定・表示名の上書きも可能
# [[repos]]
# path = "~/work/some-repo"
# name = "work-repo"
```

CLI 引数でも指定できます（設定に追加されます）:

```sh
pocket-repo ~/path/to/repo-a --port 8080
pocket-repo --help
```

### 起動とアクセス

```sh
pocket-repo
```

Tailnet 内のスマホから、開発マシンの Tailscale IP で開きます:

```
http://<開発マシンの100.x.x.x>:3000
```

常時起動（デーモン化）や Homebrew での導入手順は [PACKAGING.md](PACKAGING.md) を参照してください。

> セキュリティ: 認証は無く、Tailnet がアクセス境界です。より締めるなら `bind` を Tailscale IP に限定するか、`127.0.0.1` + `tailscale serve` を検討してください。

## 技術構成

* **言語**: Rust（単一バイナリ、静的アセットも埋め込み）
* **サーバー**: [axum](https://github.com/tokio-rs/axum) + [maud](https://maud.lambda.xyz/)（HTMLテンプレート）
* **UI フレームワーク**: [maudliver](https://github.com/ppdx999/maudliver) をベンダリング — ステートレスなサーバー駆動UI（HTTP上の Elm Architecture、ID単位のHTML差分）
* **Git**: [git2](https://github.com/rust-lang/git2-rs)（vendored libgit2、読み取り専用）
* **ハイライト**: [syntect](https://github.com/trishume/syntect)（pure-rust 正規表現）
* **ファジー検索**: [nucleo](https://github.com/helix-editor/nucleo)

リポジトリのデータは Model に持たず、`view()` が描画のたびに git から読み直す設計です。状態（展開中パス・選択ブランチなど）だけを軽量に扱い、巨大リポジトリでも「開いた部分だけ」を遅延ロードします。

## 将来的な機能

* Claude Codeセッションとの連携（「このファイルを読んで」「この差分をレビューして」といったAIへの送信）
* シンボル検索
* Git blame表示
* Pull Requestレビュー支援
* ターミナル出力の閲覧
* 複数開発マシンの管理
* プッシュ通知によるAIタスク完了通知

PocketRepoは「スマホでコードを読むためのツール」から始まり、最終的には**ポケットの中にある開発環境**を目指します。

## ライセンス

MIT
