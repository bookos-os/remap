# El binario se compila antes con cargo (empaquetar.sh): rpmbuild solo coloca
# los archivos, como en bookos-fonts y el explorador.
%global debug_package %{nil}

Name:           bookos-teclas
Version:        0.1.0
Release:        1%{?dist}
Summary:        Remapeo de teclas de BookOS con GPUI
License:        LicenseRef-BookOS
# wgpu abre Vulkan en tiempo de ejecución y el generador no lo ve.
Requires:       vulkan-loader, libxkbcommon, systemd, polkit

%description
Remapeo de teclas de BookOS con GPUI.

%install
install -Dm755 %{raiz}/target/release/bookos-teclas %{buildroot}%{_bindir}/bookos-teclas
install -Dm644 %{raiz}/data/bookos-teclas.desktop %{buildroot}%{_datadir}/applications/bookos-teclas.desktop
install -Dm644 %{raiz}/data/bookos-teclas.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/bookos-teclas.svg
install -Dm644 %{raiz}/data/bookos-teclas.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/bookos-teclas.png

install -Dm755 %{raiz}/target/servicio/release/bookos-teclasd %{buildroot}%{_bindir}/bookos-teclasd
install -Dm644 %{raiz}/../teclasd/sistema/bookos-teclasd.service %{buildroot}%{_unitdir}/bookos-teclasd.service
install -Dm644 %{raiz}/../teclasd/sistema/org.bookos.Teclas.conf %{buildroot}%{_datadir}/dbus-1/system.d/org.bookos.Teclas.conf
install -Dm644 %{raiz}/../teclasd/sistema/org.bookos.Teclas.service %{buildroot}%{_datadir}/dbus-1/system-services/org.bookos.Teclas.service
install -Dm644 %{raiz}/../teclasd/sistema/org.bookos.teclas.policy %{buildroot}%{_datadir}/polkit-1/actions/org.bookos.teclas.policy
install -Dm644 %{raiz}/../teclasd/sistema/bookos-teclasd.quirks %{buildroot}%{_datadir}/libinput/50-bookos-teclasd.quirks

%post
%systemd_post bookos-teclasd.service

%preun
%systemd_preun bookos-teclasd.service

%postun
%systemd_postun_with_restart bookos-teclasd.service

%files
%{_bindir}/bookos-teclas
%{_datadir}/applications/bookos-teclas.desktop
%{_datadir}/icons/hicolor/scalable/apps/bookos-teclas.svg
%{_datadir}/icons/hicolor/512x512/apps/bookos-teclas.png
%{_bindir}/bookos-teclasd
%{_unitdir}/bookos-teclasd.service
%{_datadir}/dbus-1/system.d/org.bookos.Teclas.conf
%{_datadir}/dbus-1/system-services/org.bookos.Teclas.service
%{_datadir}/polkit-1/actions/org.bookos.teclas.policy
%{_datadir}/libinput/50-bookos-teclasd.quirks
