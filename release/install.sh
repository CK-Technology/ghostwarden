#!/bin/bash
#
# GhostWarden Installation Script
# Installs gwarden binary, systemd units, completions, man pages, and configs
#
# Usage:
#   sudo ./install.sh              # Install everything
#   sudo ./install.sh --uninstall  # Remove everything
#   ./install.sh --help            # Show help
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="gwarden"
INSTALL_PREFIX="${INSTALL_PREFIX:-/usr}"
CONFIG_DIR="/etc/ghostwarden"
STATE_DIR="/var/lib/ghostwarden"
LOG_DIR="/var/log/ghostwarden"

# Detect project root (script should be in release/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Helper functions
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

check_dependencies() {
    local missing_deps=()

    # Check for required system packages
    if ! command -v nft &> /dev/null; then
        missing_deps+=("nftables")
    fi

    if ! command -v dnsmasq &> /dev/null; then
        print_warning "dnsmasq not found (optional, needed for DHCP/DNS)"
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        print_error "Missing required dependencies: ${missing_deps[*]}"
        echo ""
        echo "Install with:"
        echo "  Arch Linux: sudo pacman -S ${missing_deps[*]}"
        echo "  Ubuntu/Debian: sudo apt install ${missing_deps[*]}"
        exit 1
    fi
}

build_binary() {
    print_info "Building gwarden binary..."
    cd "$PROJECT_ROOT"

    if ! command -v cargo &> /dev/null; then
        print_error "Rust/Cargo not found. Install from https://rustup.rs"
        exit 1
    fi

    cargo build --release
    print_success "Binary built successfully"
}

install_binary() {
    print_info "Installing binary to ${INSTALL_PREFIX}/bin/${BINARY_NAME}..."

    if [ ! -f "$PROJECT_ROOT/target/release/$BINARY_NAME" ]; then
        print_error "Binary not found. Run build first or run with --build"
        exit 1
    fi

    install -Dm755 "$PROJECT_ROOT/target/release/$BINARY_NAME" "${INSTALL_PREFIX}/bin/${BINARY_NAME}"
    print_success "Binary installed"
}

install_systemd_units() {
    print_info "Installing systemd units..."

    install -Dm644 "$SCRIPT_DIR/systemd/gwarden.service" \
        "${INSTALL_PREFIX}/lib/systemd/system/gwarden.service"
    install -Dm644 "$SCRIPT_DIR/systemd/gwarden-metrics.service" \
        "${INSTALL_PREFIX}/lib/systemd/system/gwarden-metrics.service"

    # Reload systemd daemon
    systemctl daemon-reload

    print_success "Systemd units installed"
}

install_configs() {
    print_info "Installing configuration files..."

    # Create directories
    mkdir -p "$CONFIG_DIR/policies"
    mkdir -p "$STATE_DIR"
    mkdir -p "$LOG_DIR"

    # Install main config (don't overwrite if exists)
    if [ ! -f "$CONFIG_DIR/ghostnet.yaml" ]; then
        if [ -f "$PROJECT_ROOT/examples/ghostnet.yaml" ]; then
            install -Dm644 "$PROJECT_ROOT/examples/ghostnet.yaml" "$CONFIG_DIR/ghostnet.yaml"
            print_success "Installed example topology to $CONFIG_DIR/ghostnet.yaml"
        fi
    else
        print_warning "Config already exists at $CONFIG_DIR/ghostnet.yaml (keeping existing)"
    fi

    # Install daemon config
    if [ ! -f "$CONFIG_DIR/daemon.toml" ]; then
        install -Dm644 "$SCRIPT_DIR/configs/daemon.toml" "$CONFIG_DIR/daemon.toml"
        print_success "Installed daemon config"
    else
        print_warning "Daemon config exists (keeping existing)"
    fi

    # Install policy profiles
    if [ -d "$PROJECT_ROOT/examples/policies" ]; then
        for policy in "$PROJECT_ROOT/examples/policies"/*.yaml; do
            if [ -f "$policy" ]; then
                policy_name=$(basename "$policy")
                if [ ! -f "$CONFIG_DIR/policies/$policy_name" ]; then
                    install -Dm644 "$policy" "$CONFIG_DIR/policies/$policy_name"
                fi
            fi
        done
        print_success "Installed policy profiles"
    fi

    # Set permissions
    chmod 755 "$CONFIG_DIR"
    chmod 644 "$CONFIG_DIR"/*.yaml "$CONFIG_DIR"/*.toml 2>/dev/null || true
    chmod 755 "$STATE_DIR"
    chmod 755 "$LOG_DIR"
}

install_completions() {
    print_info "Installing shell completions..."

    # Bash
    if [ -d "${INSTALL_PREFIX}/share/bash-completion/completions" ]; then
        install -Dm644 "$SCRIPT_DIR/completions/gwarden.bash" \
            "${INSTALL_PREFIX}/share/bash-completion/completions/gwarden"
        print_success "Installed bash completion"
    fi

    # Zsh
    if [ -d "${INSTALL_PREFIX}/share/zsh/site-functions" ]; then
        install -Dm644 "$SCRIPT_DIR/completions/gwarden.zsh" \
            "${INSTALL_PREFIX}/share/zsh/site-functions/_gwarden"
        print_success "Installed zsh completion"
    fi

    # Gshell (gsh)
    if [ -d "${INSTALL_PREFIX}/share/gsh/completions" ] || [ -d "/usr/share/gsh/completions" ]; then
        local gsh_dir="${INSTALL_PREFIX}/share/gsh/completions"
        if [ ! -d "$gsh_dir" ]; then
            gsh_dir="/usr/share/gsh/completions"
        fi
        mkdir -p "$gsh_dir"
        install -Dm644 "$SCRIPT_DIR/completions/gwarden.gsh" "$gsh_dir/gwarden.gsh"
        print_success "Installed gsh (gshell) completion"
    fi
}

install_man_pages() {
    print_info "Installing man pages..."

    local man_dir="${INSTALL_PREFIX}/share/man/man1"
    mkdir -p "$man_dir"

    for manpage in "$SCRIPT_DIR/man"/*.1; do
        if [ -f "$manpage" ]; then
            install -Dm644 "$manpage" "$man_dir/$(basename "$manpage")"
        fi
    done

    # Update man database
    if command -v mandb &> /dev/null; then
        mandb -q 2>/dev/null || true
    fi

    print_success "Installed man pages"
}

post_install() {
    echo ""
    echo -e "${GREEN}============================================${NC}"
    echo -e "${GREEN}  GhostWarden Installation Complete! 🛡️${NC}"
    echo -e "${GREEN}============================================${NC}"
    echo ""
    echo "Next steps:"
    echo ""
    echo "1. Configure your network topology:"
    echo -e "   ${BLUE}sudo nano $CONFIG_DIR/ghostnet.yaml${NC}"
    echo ""
    echo "2. Preview changes:"
    echo -e "   ${BLUE}gwarden net plan${NC}"
    echo ""
    echo "3. Apply configuration (with 30s rollback window):"
    echo -e "   ${BLUE}sudo gwarden net apply --commit --confirm 30s${NC}"
    echo ""
    echo "4. (Optional) Enable services:"
    echo -e "   ${BLUE}sudo systemctl enable --now gwarden-metrics.service${NC}"
    echo ""
    echo "5. Run diagnostics:"
    echo -e "   ${BLUE}sudo gwarden doctor${NC}"
    echo ""
    echo "6. Launch TUI dashboard:"
    echo -e "   ${BLUE}gwarden tui${NC}"
    echo ""
    echo "Documentation:"
    echo -e "   ${BLUE}man gwarden${NC}"
    echo ""
    echo "Configuration files:"
    echo "   • Topology: $CONFIG_DIR/ghostnet.yaml"
    echo "   • Daemon:   $CONFIG_DIR/daemon.toml"
    echo "   • Policies: $CONFIG_DIR/policies/"
    echo ""
}

uninstall() {
    print_info "Uninstalling GhostWarden..."

    # Stop services
    if systemctl is-active --quiet gwarden.service; then
        systemctl stop gwarden.service
    fi
    if systemctl is-active --quiet gwarden-metrics.service; then
        systemctl stop gwarden-metrics.service
    fi

    # Disable services
    systemctl disable gwarden.service 2>/dev/null || true
    systemctl disable gwarden-metrics.service 2>/dev/null || true

    # Remove binary
    rm -f "${INSTALL_PREFIX}/bin/${BINARY_NAME}"
    print_success "Removed binary"

    # Remove systemd units
    rm -f "${INSTALL_PREFIX}/lib/systemd/system/gwarden.service"
    rm -f "${INSTALL_PREFIX}/lib/systemd/system/gwarden-metrics.service"
    systemctl daemon-reload
    print_success "Removed systemd units"

    # Remove completions
    rm -f "${INSTALL_PREFIX}/share/bash-completion/completions/gwarden"
    rm -f "${INSTALL_PREFIX}/share/zsh/site-functions/_gwarden"
    rm -f "${INSTALL_PREFIX}/share/gsh/completions/gwarden.gsh"
    rm -f "/usr/share/gsh/completions/gwarden.gsh"
    print_success "Removed shell completions"

    # Remove man pages
    rm -f "${INSTALL_PREFIX}/share/man/man1/gwarden"*.1
    if command -v mandb &> /dev/null; then
        mandb -q 2>/dev/null || true
    fi
    print_success "Removed man pages"

    echo ""
    print_warning "Configuration preserved in:"
    echo "  • $CONFIG_DIR"
    echo "  • $STATE_DIR"
    echo ""
    echo "To completely remove all data:"
    echo -e "  ${BLUE}sudo rm -rf $CONFIG_DIR $STATE_DIR $LOG_DIR${NC}"
    echo ""
    print_success "GhostWarden uninstalled"
}

show_help() {
    cat << EOF
GhostWarden Installation Script

Usage:
    sudo ./install.sh [OPTIONS]

Options:
    --build         Build binary before installing
    --uninstall     Remove GhostWarden from system
    --help          Show this help message

Environment Variables:
    INSTALL_PREFIX  Installation prefix (default: /usr)

Examples:
    # Full installation (build + install)
    sudo ./install.sh --build

    # Install pre-built binary
    sudo ./install.sh

    # Uninstall
    sudo ./install.sh --uninstall

    # Install to /usr/local instead of /usr
    sudo INSTALL_PREFIX=/usr/local ./install.sh

Installation Directories:
    Binary:      ${INSTALL_PREFIX}/bin/gwarden
    Systemd:     ${INSTALL_PREFIX}/lib/systemd/system/
    Config:      $CONFIG_DIR
    State:       $STATE_DIR
    Logs:        $LOG_DIR
    Completions: ${INSTALL_PREFIX}/share/{bash-completion,zsh,gsh}/
    Man pages:   ${INSTALL_PREFIX}/share/man/man1/

EOF
}

# Main installation flow
main() {
    local do_build=false
    local do_uninstall=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --build)
                do_build=true
                shift
                ;;
            --uninstall)
                do_uninstall=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Check if running as root
    check_root

    if [ "$do_uninstall" = true ]; then
        uninstall
        exit 0
    fi

    # Check dependencies
    check_dependencies

    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  GhostWarden Installation${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
    echo "Install prefix: $INSTALL_PREFIX"
    echo "Config dir:     $CONFIG_DIR"
    echo "State dir:      $STATE_DIR"
    echo ""

    # Build if requested
    if [ "$do_build" = true ]; then
        build_binary
    fi

    # Install components
    install_binary
    install_systemd_units
    install_configs
    install_completions
    install_man_pages

    # Post-install message
    post_install
}

# Run main
main "$@"
