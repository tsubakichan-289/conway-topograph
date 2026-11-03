# 検証記録

実施日: 2026-10-12。Rust 1.95.0 / Cargo 1.95.0、Linux NixOS。

## 各 Phase

| Phase | 実装内容 | 検証 |
| --- | --- | --- |
| 1 | BigInt 形式、原始ベクトル、unimodular pair、Farey flip、厳密な係数復元 | コアの fmt / clippy / test / check 成功。依存取得後、GUIを含む同4チェックも成功 |
| 2 | Cayley・内心・自己同型・測地線・面領域の描画 | workspace の fmt / clippy / test / check 成功 |
| 3 | マウス移動・選択・編集・固定面 | workspace の fmt / clippy / test / check 成功 |
| 4 | 正負定値の井戸、二重井戸、川、湖、特徴探索・川の追跡 | workspace の fmt / clippy / test / check 成功 |
| 5 | 予算と枝刈り、隣接・座標・シーン・面三角形分割キャッシュ、JSON、SVG/PNG | workspace の fmt / clippy / test / check 成功。release ビルド成功 |

各 Phase の開発中に検出したコンパイル・lint・テスト失敗は修正し、同4チェックを再実行した。
最終チェックは `scripts/check.sh`（キャッシュ利用の `--offline`、clippy は `-D warnings`）。

## テスト

合計21テスト成功、失敗・無視は0。

| 依頼の検証項目 | テストファイル・主な条件 |
| --- | --- |
| 直接評価 / 判別式 | core/tests/algebra.rs：指定4形式、150桁級の整数、各辺の局所判別式と全域判別式の一致 |
| 原始ベクトル / unimodular pair | core/tests/algebra.rs：符号正規化、非原始・零ベクトル拒否、行列式の絶対値 |
| 伝播則 | core/tests/algebra.rs：深さ5の双対木、複数の正負・混合項形式 |
| 初期3面 / 任意3面の復元 | core/tests/algebra.rs：Cramer法、従属・非整数解の拒否 |
| Cayley / 円板の等長変換 | core/tests/geometry.rs：往復、円板保存、距離保存、ドラッグのアンカー、10000回の合成 |
| Farey三角形 / 双対木 | core/tests/algebra.rs・geometry.rs：反転の可逆性、三価・無閉路・ノード数、内心の等距離、局所基底変更 |
| 井戸 / 川 / 湖 | core/tests/features.rs：3方向の変化、正負定値、単一・二重井戸、非平方の川64辺、平方の有限separator、退化・零形式 |
| 描画範囲 | core/tests/scene.rs：深さ・頂点予算、境界内の座標、面内部の選択、キャッシュ再利用 |
| 編集の確定 | core/tests/editor.rs：他固定値保持、失敗時の確定形式保持 |
| JSON / PNG・SVG | tests/storage_export.rs：巨大整数保存、無効設定拒否、PNG復号、SVGの円弧・clipPath・特徴色 |
| 配色の保存・互換性・出力 | tests/storage_export.rs：選択色のJSON往復、旧JSON読み込み、SVGの線・文字・枠色、PNGの円板内外の画素色 |
| 値による配色モード | tests/value_colors.rs：正負対称性、暗くなる単調性、10000桁の整数、井戸・選択面の符号色保持、旧設定の互換性、PNGの赤・青・湖・黒の画素色 |
| egui 操作 | src/app.rs：ウィンドウなしの実 egui フレームで、面クリック・ドラッグ・リセット |

4例の PNG/SVG を `/tmp/topograph-examples` に生成し、井戸・川・湖の画像を目視確認した。
画像出力では無限の湖の表示範囲が有限の面パッチの集まりとして表れる。

## 環境上の問題と未確認事項

ホスト側の crates.io DNS / HTTPS はユーザーにより正常と確認された。
Codex 内の getent / curl は名前解決に失敗し、sandbox 外の接続確認もこのセッションの
`sandbox_approval=false` によりツール側で拒否された。依存関係はユーザーがホストから
`.cargo-home` へ取得し、その後のコンパイル・テストは Codex 内でオフライン実行できた。

`cargo run --release --offline` は release 実行ファイルを起動したが、winit の Wayland 初期化で
`WaylandError(Connection(NoCompositor))` を返した。
このため、**Codex 内での実ウィンドウ表示・GPUドライバ動作は未確認**。
egui の UI・選択・ドラッグのテストは compositor を使わず実施した。
`shell.nix` の評価も sandbox から Nix daemon socket に接続できず、ホストでの確認が必要。
システム設定・DNS・NetworkManager・configuration.nix は変更していない。

## 数学的・機能的な限界

整数モデルと局所の判定条件は文書化・テスト済み。
無限木全体の探索、任意の巨大係数での有限ステップ以内の特徴発見は保証しない。
特徴探索は4096ステップ、描画は局所深さと2500頂点で制限する。
任意の極端な視点における f64 の誤差上限を形式証明してはいない。
双曲測地線の画面描画と湖の境界は画素用の有限分割であり、境界の無限解像度は提供しない。
川の全周期・Pell の基本解の計算は未実装（今回の必須機能の範囲外）。

## UI 簡素化の追記

ホストでの起動確認依頼に対し、ユーザーから画面の肯定的なフィードバックとUI簡素化の希望があった。
例のプリセット・初期設定へ戻すボタン・キャッシュ件数などの常時表示を除き、
固定面・表示調整・特徴探索・保存出力・面の詳細を折りたたむ構成に変更した。
下部の選択パネルは面を選んだときにだけ表示する。
簡素化後の画面でも egui の面選択・ドラッグテストを実施。
