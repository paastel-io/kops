# kops

1. setup requirements (k3d, rust, kubectl, docker)
   sudo groupadd --system kopsd
   sudo useradd --system --no-create-home --gid kopsd kopsd
   sudo usermod -aG kopsd $USER
2. create cluster
3. run daemon
4. run ctrl
