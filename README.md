# kops

1. setup requirements (k3d, rust, kubectl, docker)
   sudo groupadd kopsd
   sudo useradd --system --no-create-home --gid kopsd kopsd
   sudo usermod -aG kopsd $USER
   sudo mkdir -p /var/run/kopsd
   sudo chown root:kopsd /var/run/kopsd
   sudo chmod 0770 /var/run/kopsd
2. create cluster
3. run daemon
4. run ctrl
