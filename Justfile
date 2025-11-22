default:
    @just --list

# Setups

# Dry-run setup (do not install)
setup-dry:
    ./scripts/setup.sh --dry-run

# K3D cluster management

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
