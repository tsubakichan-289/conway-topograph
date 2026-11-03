# Conway Topograph

A Rust / eframe desktop application for exploring integer binary quadratic forms through Conway topographs. It displays hyperbolic incenters of Farey triangles and their dual edges in the Poincaré disk.

The combinatorial structure, region labels, and reconstruction of coefficients use exact `BigInt` arithmetic. Numerical labels belong to regions, rather than to vertices of the tree.

## Getting started

Requires Rust 1.95 or later and a Linux desktop with Wayland or X11 and OpenGL.

```sh
git clone https://github.com/tsubakichan-289/conway-topograph.git
cd conway-topograph
cargo run --release
```

On NixOS, use the supplied development shell:

```sh
nix-shell shell.nix
cargo run --release
```

If you already have a populated project-local Cargo cache:

```sh
export CARGO_HOME="$PWD/.cargo-home"
nix-shell shell.nix --run 'cargo run --release --offline'
```

## Exploring a form

Enter coefficients and select **Apply coefficients**. The interface initially focuses on coefficient input and view reset; open **Fixed faces**, **Display**, **Colors**, **Explore**, and **Export / files** as needed. There are no preset forms.

| Action | Effect |
| --- | --- |
| Left-button drag | Move the viewpoint using a hyperbolic isometry of the disk |
| Mouse wheel | Change the disk's display scale between 35% and 100% |
| Click a region | Inspect its primitive vector, exact value, and visible neighbors |
| Double-click | Move the hyperbolic viewpoint to the clicked position |
| **Reset view** | Return to the initial Farey triangle and viewpoint |
| **Find well / river / lake** | Follow exact integer descent, up to 4,096 steps |
| **River next / Reverse** | Move one vertex along the boundary between nonzero regions of opposite signs |

The selected region's editor appears below the disk. **Face details** shows adjacency information and the evidence used to identify wells.

### Editing region labels

The initial fixed regions correspond to `[1, 0]`, `[0, 1]`, and `[1, 1]`, with labels `A`, `C`, and `S`. **Apply three labels** reconstructs the form exactly as `a = A`, `c = C`, and `b = S - A - C`.

To use other regions, select a region and press **Fix this face**. Use **Replace** or **Remove** if three fixed regions are already present. Enter all three labels and press **Apply three labels**. **Apply selected label** changes only the selected region's label while retaining the other two fixed values.

Nonunique solutions, noninteger coefficients, and insufficient fixed regions produce an error without replacing the committed form. Direct coefficient changes also update the fixed-region labels.

### Colors and graph layers

**Colors** controls the disk background, outer canvas, ordinary edges and vertices, region text, and disk boundary. Changes apply immediately and are included in saved settings and exports.

**Display → Value colors** colors positive regions red and negative regions blue. Colors approach black as the absolute value increases. **Color scale** is the absolute value at which brightness reaches half its maximum. Zero-valued regions use the lake color, or the background color when lake display is disabled. Text colors adapt to the region background.

**Display → Graph** offers three views:

- **Topograph**: the trivalent tree.
- **Dual**: the corresponding Farey triangles.
- **Overlay**: both structures together.

Dual edges meet the corresponding tree edges perpendicularly at their hyperbolic midpoints. Triangle vertices lie on the disk boundary. All three sides of a triangle are drawn even at the edge of the generated view. Region coloring, labels, and selection retain their original region correspondence. Use the **Normal** color mode and disable annotations for a line-only view. In overlay mode, triangles use the **Disk boundary** color.

### Wells, rivers, and lakes

Rivers are highlighted in blue, wells in gold, and lakes in light blue. **Explore → Well evidence** shows the three directional changes in the form's value and the region values. Negative definite forms use ascent for the negated form. Flat edges of double wells are highlighted as well.

The application distinguishes lakes and their finite separator for square discriminants from the infinite river for nonsquare discriminants. See the [mathematical model and references](docs/MATHEMATICS.md) for definitions and derivations; this document is currently in Japanese.

## Saving and exporting

The committed coefficients, fixed regions, local basis, viewpoint, and display settings are automatically saved as JSON in eframe's storage under `conway-exact-v1`. This key is separate from the legacy manual editor. Uncommitted input is not saved.

**Save JSON** and **Load JSON** also support explicit files. Invalid settings or inconsistent labels are rejected.

The export path uses the appropriate `.svg`, `.png`, or `.json` extension. Existing destination files are overwritten. PNG and SVG exports are 1600 × 1600 disk diagrams reflecting the current finite view, selected region, and display options. SVG uses circular arcs for dual edges, or straight lines for diameters. PNG is rendered from the same SVG with resvg and egui's Hack font for labels.

Generate examples without opening a GUI:

```sh
cargo run --example render_examples -- /tmp/topograph-examples
cargo run --example render_dual -- /tmp/topograph-dual
```

The first command exports four examples. The second compares the ordinary, dual, and overlay views.

## Project structure

| Location | Responsibility |
| --- | --- |
| `core/src/algebra.rs` | Forms, primitive vectors, determinants, coefficient reconstruction |
| `core/src/topology.rs` | Farey triangles and the trivalent dual tree |
| `core/src/geometry.rs` | Cayley transform, incenters, geodesics, local coordinates, isometries |
| `core/src/features.rs` | Wells, rivers, lakes, exact descent |
| `core/src/scene.rs` | Visible branches, pixel-based pruning, budgets, reusable caches |
| `core/src/editor.rs` | Three-region edits and preservation of state on errors |
| `src/app.rs`, `src/render.rs` | egui interface, drawing, interaction |
| `src/storage.rs`, `src/export.rs` | JSON persistence and image exports |
| `legacy/` | Previous manual node editor, retained for reference |

## Development checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
```

Add `--offline` when dependencies are already cached. To run only the GUI-independent core tests:

```sh
cargo test -p topograph-core
```

Tests cover algebra, topology-related behavior, geometry, feature detection, editing, JSON validation, PNG decoding, and egui region selection and dragging. The [verification notes](docs/VERIFICATION.md), currently in Japanese, distinguish completed checks from environment-dependent GUI validation.

## Limitations

Rendering is finite: local depth is limited to 8–64, with at most 2,500 vertices and additional pixel-size pruning. Panning regenerates only the required area and reuses adjacency and incenter data. The cache is pruned on the next update after exceeding 24,000 nodes. Very small boundary branches and labels that do not fit are omitted. Lake coloring is restricted to generated region patches.

Geometry uses `f64`, screen coordinates use `f32`, and curves are approximated by finite pixel segments. Recomputing the integer local basis during deep navigation reduces cancellation, but the disk boundary does not have infinite resolution.

Finite exploration does not guarantee complete river periods, fundamental Pell solutions, or a global classification of all features. Very large coefficients can reach the descent limit, in which case the application reports an error.

---

[日本語版 README](README.ja.md)
