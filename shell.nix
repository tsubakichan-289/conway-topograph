{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  packages = with pkgs; [ cargo rustc rustfmt clippy pkg-config ];
  buildInputs = with pkgs; [ wayland libxkbcommon libGL libglvnd xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [ wayland libxkbcommon libGL libglvnd xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr ]);
}
