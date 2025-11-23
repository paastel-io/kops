alias kd := kopsd
alias kc := kopsctl

default:
    @just --list

#
# Setup
#

# Dry-run setup (do not install)
setup-dry:
    ./scripts/setup.sh --dry-run

#
# Cargo
#

# Install kopsd and kopctl
install:
    just install-kopsd
    just install-kopsctl

# Install kopsd
install-kopsd:
    cargo install --path kopsd

# Install kopctl
install-kopsctl:
    cargo install --path kopsctl

# Audit deny
audit:
    cargo deny check

# Run kops daemon
kopsd:
    cargo kopsd

# Run kops control
kopsctl:
    cargo kopsctl

#
# K3D cluster management
#

# Create a k3d cluster
cluster-create CLUSTER:
    @echo "Creating k3d cluster: {{ CLUSTER }}"
    k3d cluster start {{ CLUSTER }}

# Start a k3d cluster
cluster-up CLUSTER:
    @echo "Creating k3d cluster: {{ CLUSTER }}"
    k3d cluster start {{ CLUSTER }}

# Stop a cluster
cluster-down CLUSTER:
    @echo "Stopping k3d cluster: {{ CLUSTER }}"
    k3d cluster stop {{ CLUSTER }}

# Delete a cluster
cluster-rm CLUSTER:
    @echo "Deleting k3d cluster: {{ CLUSTER }}"
    k3d cluster delete {{ CLUSTER }}

# Show cluster info
cluster-info CLUSTER:
    @echo "Info k3d cluster: {{ CLUSTER }}"
    kubectl config get-contexts k3d-{{ CLUSTER }}
    kubectl get nodes
    kubectl get pods -A
