{pkgs}: {
  deps = [
    pkgs.clang
    pkgs.gcc
    pkgs.wayland-protocols
    pkgs.wayland
    pkgs.pkg-config
    pkgs.qt5.qtx11extras
    pkgs.qt5.qtbase
    pkgs.cmake
    pkgs.xorg.libXrandr
    pkgs.xorg.libXext
    pkgs.xorg.libxcb
    pkgs.xorg.libXtst
    pkgs.xorg.libXi
    pkgs.xorg.libX11
    pkgs.leptonica
    pkgs.tesseract
    pkgs.pipewire
    pkgs.gst_all_1.gst-plugins-bad
    pkgs.gst_all_1.gst-plugins-good
    pkgs.gst_all_1.gst-plugins-base
    pkgs.gst_all_1.gstreamer
    pkgs.gtk4-layer-shell
    pkgs.gtk4
  ];
}
