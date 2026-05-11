service_name := "traccar"

[private]
default:
	just --list

# Install the current state as a user-local binary
[group('Setup')]
install:
    cargo install --path .
# Install the user-local binary as a user-local service
[group('Service')]
service-install:
    cp ./traccar.service ~/.config/systemd/user/{{service_name}}.service
    systemctl --user daemon-reload

# Remove the service
[group('Service')]
service-uninstall:
    systemctl --user stop {{service_name}}.service
    systemctl --user disable {{service_name}}.service
    rm -i -v ~/.config/systemd/user/{{service_name}}.service
    systemctl --user daemon-reload

