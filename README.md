# IME

軽量で、インストール後すぐ使える日本語IMEを目指すプロジェクトです。現在はmacOSを実装対象とし、変換コアは将来WindowsやLinuxでも共有できるようRustで開発しています。

## 現在の実装

- incrementalなローマ字→ひらがな変換
- composition、変換、候補選択、確定、取消の状態機械
- 小さな組み込み辞書
- 読みラティス上の最小cost経路探索
- Swiftなどのnative adapterから呼べるC ABI
- unit、golden、C ABI smoke test
- release profileで動くmicro benchmark

辞書は動作確認用の数語だけです。現時点の候補品質やbenchmark値は、製品品質を表すものではありません。

## 必要環境

- Rust stable最新版（現在は1.97.1、`rust-toolchain.toml`でstableを追従）
- [just](https://github.com/casey/just)
- macOSのC ABI smoke testにはClang

macOSで`just`が未導入の場合は、Homebrewで導入できます。

```sh
brew install just
```

## 最短の使い方

macOSでは開発版をユーザー領域へインストールし、ほかのアプリへ直接入力できます。

```sh
just install-macos
```

初回インストール直後にmacOSが入力ソースを認識しない場合は、ログアウトして再ログインしたあと次を実行します。

```sh
just select-macos
```

ユーザー領域の開発版をmacOSが認識しない場合は、管理者パスワードの確認後、システム領域へインストールできます。

```sh
just install-macos-system
```

`Unvalley IME`へ切り替わったらTextEditなどを開き、次の順で試してください。

1. `nihon`と入力すると`にほん`がpreedit表示される
2. Spaceで`日本`へ変換する
3. Spaceを繰り返して候補を切り替える
4. Enterで確定する

現在の辞書は動作確認用の数語だけで、候補一覧ウィンドウはまだありません。Spaceを押すたびに選択中の候補がpreedit上で切り替わります。

まず開発環境を確認し、全テストを実行します。

```sh
just doctor
just check
```

短時間のベンチマークは次のコマンドで実行できます。

```sh
just bench-smoke
```

Swift製macOSアダプターから接続するためのC ABIライブラリは、次のコマンドで生成できます。

```sh
just build-ffi
```

生成物は`target/release/libime_ffi.dylib`、Cヘッダーは`crates/ime-ffi/include/ime_ffi.h`です。

利用可能なコマンドの一覧は、引数なしの`just`で表示できます。

```sh
just
```

## よく使うコマンド

```sh
just fmt          # コードを整形
just test         # Rustテスト
just test-ffi     # C ABI smoke test
just check        # format、lint、全テスト
just test-macos   # Swift→Rust接続テスト
just check-macos  # macOS bundleを含む全検証
just install-macos # build・検証・インストール・選択
just install-macos-system # /Library/Input Methodsへ管理者インストール
just select-macos # インストール済み入力ソースへ切り替え
just bench        # 通常のbenchmark
just bench-smoke  # 短時間のbenchmark
just ci           # CI相当の検証
```

計測方法と初回baselineは[docs/benchmarking.md](./docs/benchmarking.md)を参照してください。

## 設計資料

- [既存IMEの調査と開発方針](./docs/ime-research-and-direction.md)
- [かな漢字変換・入力研究レビュー](./docs/kana-kanji-conversion-literature-review.md)
