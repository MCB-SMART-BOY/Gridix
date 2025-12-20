Name:           gridix
Version:        1.0.0
Release:        1%{?dist}
Summary:        Fast, secure database management tool with Helix/Vim keybindings

License:        MIT
URL:            https://github.com/MCB-SMART-BOY/Gridix
Source0:        https://github.com/MCB-SMART-BOY/Gridix/releases/download/v%{version}/gridix-linux-x86_64.tar.gz

ExclusiveArch:  x86_64

BuildRequires:  gtk3
Requires:       gtk3
Requires:       xdotool

%description
Gridix is a fast, secure, cross-platform database management tool with 
Helix/Vim-style keybindings. It supports SQLite, PostgreSQL, and MySQL 
databases with features like SSH tunneling, SSL/TLS encryption, and 
19 built-in themes.

%prep
%setup -q -c

%install
mkdir -p %{buildroot}%{_bindir}
install -m 755 gridix %{buildroot}%{_bindir}/gridix

mkdir -p %{buildroot}%{_datadir}/applications
cat > %{buildroot}%{_datadir}/applications/gridix.desktop << 'DESKTOP'
[Desktop Entry]
Name=Gridix
Comment=Database management tool with Helix/Vim keybindings
Exec=gridix
Icon=gridix
Terminal=false
Type=Application
Categories=Development;Database;
Keywords=database;sql;sqlite;postgresql;mysql;
DESKTOP

%files
%{_bindir}/gridix
%{_datadir}/applications/gridix.desktop

%changelog
* Fri Dec 20 2024 MCB-SMART-BOY <mcb2720838051@gmail.com> - 1.0.0-1
- Major release v1.0.0
- Enhanced ER diagram with column details (NULL/NOT NULL, default values)
- Improved SQL editor autocomplete
- Many UI/UX improvements

* Fri Dec 20 2024 MCB-SMART-BOY <mcb2720838051@gmail.com> - 0.5.2-1
- Fix LICENSE file missing in release tag
- Update version to 0.5.2

* Fri Dec 20 2024 MCB-SMART-BOY <mcb2720838051@gmail.com> - 0.5.1-1
- Initial package release
- Added AUR packages
- Added AppImage support

