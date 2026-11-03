# Conway Topograph

整数係数二元二次形式を Conway topograph として観察する Rust / eframe デスクトップアプリです。
組合せ構造・ラベル・係数復元は BigInt による厳密計算です。
Farey 三角形の双曲内心と、その双対辺を Poincaré 円板上に表示します。
面の数値は面領域内に表示し、木の頂点の番号としては表示しません。

## 起動

Rust stable 1.95 以降、Linux の Wayland または X11 / OpenGL デスクトップ環境が必要です。

```sh
git clone https://github.com/tsubakichan-289/conway-topograph.git
cd conway-topograph
cargo run --release
```

NixOS では開発用ライブラリを含むシェルを利用してください。

```sh
nix-shell shell.nix
cargo run --release
```

この作業環境の Cargo キャッシュを使う場合：

```sh
export CARGO_HOME="$PWD/.cargo-home"
nix-shell shell.nix --run 'cargo run --release --offline'
```

## 操作

係数と視点リセットを中心にした簡素なUIです。形式のプリセットは設けていません。
`Fixed faces`・`Display`・`Colors`・`Explore`・`Export / files` は必要なときだけ開きます。
面を選ぶと下部の編集欄が現れ、隣接関係・井戸の判定根拠は `Face details` で確認できます。

`Colors` では円板の背景・外側のキャンバス背景・通常の線と頂点・面の文字・円板の枠を選べます。
色見本をクリックしてカラーピッカーを開いてください。変更は即座に反映され、自動保存とJSON保存、PNG/SVG出力にも引き継がれます。
川・井戸・湖の強調色は、それぞれ青・金・水色です。

`Display` の `Value colors` は、面の値の符号と絶対値で塗るモードです。
正の値を赤、負の値を青で表示し、絶対値が増えるほど黒に近づけます。
`Color scale` を大きくすると、より大きな値まで明るく表示します。明るさが半分になる絶対値がこの数値です。
零値の面は湖として水色に表示し、湖の表示をOFFにした場合は背景色にします。
文字は面の色に合わせて読みやすい明暗に調整します。
このモードでは井戸・選択面の塗りつぶしも赤青の規則を守り、井戸や川の線・頂点の強調と選択面の境界で位置を確認できます。
モードとスケールは自動保存・JSON・PNG/SVG出力に反映されます。

`Display` の `Graph` で描画するグラフを切り替えられます。
`Topograph` は従来の木、`Dual` はその双対となる Farey 三角形、`Overlay` は両者の重ね合わせです。
三角形の各辺は、元の辺を双曲距離で二等分し、直角に交わる円弧（または直径）です。
三角形の頂点は円板の境界上にあり、表示範囲の末端でも三角形の3辺を描きます。
面の着色・ラベル・面選択は従来の領域に対応します。線だけを見たい場合は配色を `Normal` にして各表示チェックをOFFにしてください。
重ね合わせでは三角形を `Disk boundary` の色で描きます。
グラフの切り替えも自動保存・JSON・PNG/SVG出力に反映されます。

- 左ドラッグ：円板の自己同型による双曲的な視点移動。
- ホイール：円板の表示スケール（35–100%）。円板全体がキャンバス内に収まります。
- 面クリック：原始ベクトル、厳密値、表示中の隣接面・辺・頂点を選択して下部に表示。
- ダブルクリック：クリック位置へ双曲的に視点を移動。
- `Reset view`：初期 Farey 三角形と視点に戻る。
- `Find well / river / lake`：整数の descent を辿って特徴を探す（4096ステップ上限）。
- `River next` / `Reverse`：非零異符号の共有面を持つ辺に沿って1頂点ずつ移動。

係数の入力を `Apply coefficients` で確定します。初期固定面の入力欄は
[1,0], [0,1], [1,1] に対応する A,C,S です。
`Apply three labels` は a=A, c=C, b=S−A−C に相当する厳密な復元を行います。

任意の面を編集するには、その面をクリックして `Fix this face` を押します。
既に3面ある場合は固定欄の `Replace` で選択面と置き換えるか、`Remove` で減らします。
3面を揃えてから各ラベルを入力し `Apply three labels` を押してください。
下部の `Apply selected label` は選択面のみを変更し、他の2固定面を保持します。
非一意・非整数係数・固定面不足の場合はエラーを表示し、確定済みの形式を保持します。
係数を直接変更したときは固定面のラベルも新形式の評価に更新します。

川は青の太線、井戸は金色、湖は水色の面領域です。
`Explore` 内の `Well evidence` に3方向の ΔQ と面の値を表示します。
負定値では −Q の ascent を用います。二重井戸の平坦辺も強調します。
平方判別式の湖と、その間の有限 separator は、非平方判別式の無限の川から区別しています。
[数学モデルと出典](docs/MATHEMATICS.md)に条件と導出を記載しています。

## 保存・出力

設定と確定した係数・固定面・局所基底・視点・表示切替を JSON として eframe の保存領域に自動保存します。
旧手動エディタとは別のキー `conway-exact-v1` を使います。
`Save JSON` / `Load JSON` でファイルにも保存・復元できます。無効な設定や矛盾したラベルは拒否します。
未確定の入力欄の内容は保存対象外です。

パス欄の拡張子を `.svg` / `.png` / `.json` に置き換えて出力します。
指定したファイルが存在する場合は上書きします。
PNG/SVG は現在の有限描画範囲・選択面・表示切替を反映する1600×1600の円板図です。
SVG の双対辺は境界直交円の arc コマンド（直径は line）です。
PNG は同じ SVG を resvg で描画し、ラベルに egui の Hack フォントを使用します。

4つの例を GUI 接続なしで出力できます。

```sh
cargo run --example render_examples -- /tmp/topograph-examples
```

線だけの通常表示・双対グラフ・重ね合わせの比較画像は
`cargo run --example render_dual -- /tmp/topograph-dual` で出力できます。

## 構成・検証

- `core/src/algebra.rs`：形式、原始ベクトル、行列式、係数復元。
- `core/src/topology.rs`：Farey 三角形と三価双対木。
- `core/src/geometry.rs`：Cayley、内心、測地線、局所座標と等長変換。
- `core/src/features.rs`：井戸、川、湖、厳密な descent。
- `core/src/scene.rs`：可視枝の生成、画素枝刈り、有限予算、再利用キャッシュ。
- `core/src/editor.rs`：3固定面による変更の確定とエラー時の保持。
- `src/app.rs` / `render.rs`：egui UI・描画・操作。
- `src/storage.rs` / `export.rs`：JSON・SVG・PNG。
- `legacy/`：従来の手動ノードエディタを参照用に保存。

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
```

依存クレートが既にキャッシュにある場合は各コマンドに `--offline` を追加できます。
GUI 非依存のコアだけの実行は `cargo test -p topograph-core`。
テストは依頼の12項目に加え、変更の確定、JSON検証、実際のPNG復号、eguiの面選択・ドラッグを含みます。

## 制限

有限の表示を、局所深さ（8–64）、最大2500頂点、画素サイズで制限します。
パン中も必要な範囲だけを再生成し、既存の隣接関係と内心座標を再利用します。
キャッシュは24000ノードを超えた次の更新で整理します。
境界の極小枝や文字が入らない領域は省略します。省略した面の情報は移動や下部の情報表示で確認してください。
湖の着色も生成済みの面パッチに限ります。

幾何座標は f64、画面座標は f32 です。曲線の画素分割は有限近似です。
深い移動では整数局所基底を取り直して桁落ちを抑えますが、円板境界での無限解像度は提供しません。
川の全周期・Pell の基本解、特徴の全域分類を有限探索で保証する機能は実装していません。
非常に大きい係数では descent の上限に達し得ます。その場合はエラーを表示します。

[検証記録](docs/VERIFICATION.md)で、実施済みチェックと実ウィンドウ起動の環境制約を区別しています。
