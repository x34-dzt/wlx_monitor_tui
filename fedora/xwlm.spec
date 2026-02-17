Name:           xwlm
Version:        0.1.0
Release:        1%{?dist}
Summary:        A TUI for managing Wayland monitor configurations

License:        MIT
URL:            https://github.com/x34-dzt/wlx_monitor_tui
Source0:        %{url}/archive/v%{version}/wlx_monitor_tui-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  wayland-devel

%description
A terminal user interface for managing Wayland monitor configurations.
Supports Hyprland, Sway, and River compositors.

%prep
%autosetup -n wlx_monitor_tui-%{version}

%build
cargo build --release

%install
install -Dm755 target/release/xwlm %{buildroot}%{_bindir}/xwlm
install -Dm644 LICENSE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE

%files
%license LICENSE
%{_bindir}/xwlm
