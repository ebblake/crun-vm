# 2. Using crun-qemu as a Podman or Docker runtime

Here we overview some of the major features provided by crun-qemu.

To run the examples below using Docker instead of Podman, you must additionally
pass `--security-opt label=disable` to docker-run. Other than that, and unless
otherwise stated, you can simply replace `podman` with `docker` in the commands
below.

## Booting VMs

### From regular VM image files

First, obtain a QEMU-compatible VM image and place it in a directory by itself:

```console
$ mkdir my-vm-image
$ curl -LO --output-dir my-vm-image https://download.fedoraproject.org/pub/fedora/linux/releases/39/Cloud/x86_64/images/Fedora-Cloud-Base-39-1.5.x86_64.qcow2
```

Then run:

> This example does not work with Docker, as docker-run does not support the
> `--rootfs` flag; see the next section for a Docker-compatible way of running
> VM images.

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    --rootfs my-vm-image \
    ""  # unused, but must specify command
```

The VM console should take over your terminal. To abort the VM, press `ctrl-]`.

You can also detach from the VM without terminating it by pressing `ctrl-p,
ctrl-q`. Afterwards, reattach by running:

> For this command to work with Docker, you must replace the `--latest` flag
> with the container's name or ID.

```console
$ podman attach --latest
```

This command also works when you start the VM in detached mode using
podman-run's `-d`/`--detach` flag.

It is also possible to omit flags `-i`/`--interactive` and `-t`/`--tty` to
podman-run, in which case you won't be able to interact with the VM but can
still observe its console. Note that pressing `ctrl-]` will have no effect, but
you can always use the following command to terminate the VM:

> For this command to work with Docker, you must replace the `--latest` flag
> with the container's name or ID.

```container
$ podman stop --latest
```

Changes made by the VM to its image are by default not persisted in the original
image file. This can be changed by passing in the non-standard option
`--persist-changes` *after* the `--rootfs` option:

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    --rootfs my-vm-image \
    --persist-changes
```

> :warning: When using `--persist-changes`, make sure that the image file is
> never simultaneously used by another process or VM, otherwise **data
> corruption may occur**.

### From VM image files packaged into container images

crun-qemu also works with container images that contain a VM image file with
any name under `/` or under `/disk/`. No other files may exist in those
directories. Containers built for use as [KubeVirt `containerDisk`s] follow this
convention, so you can use those here:

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    quay.io/containerdisks/fedora:39 \
    ""  # unused, but must specify command because container image does not
```

You can also use `util/package-vm-image.sh` to easily package a VM image into a
container image, and `util/extract-vm-image.sh` to extract a VM image contained
in a container image.

Note that flag `--persist-changes` has no effect when running VMs from container
images.

## First-boot customization

### cloud-init

In the examples above, you were able to boot the VM but not to log in. To fix
this and do other first-boot customization, you can provide a [cloud-init]
NoCloud configuration to the VM by passing in the non-standard option
`--cloud-init` *after* the image specification:

> For this command to work with Docker, you must provide an absolute path to
> `--cloud-init`.

```console
$ ls examples/cloud-init/config/
meta-data  user-data  vendor-data

$ podman run \
    --runtime crun-qemu \
    -it --rm \
    quay.io/containerdisks/fedora:39 \
    --cloud-init examples/cloud-init/config
```

You should now be able to log in with the default `fedora` username and password
`pass`.

### Ignition

Similarly, you can provide an [Ignition] configuration to the VM by passing in
the `--ignition` option:

> For this command to work with Docker, you must provide an absolute path to
> `--ignition`.

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    quay.io/crun-qemu/example-fedora-coreos:39 \
    --ignition examples/ignition/config.ign
```

You should now be able to log in with the default `core` username and password
`pass`.

## SSH'ing into the VM

Assuming the VM supports cloud-init or Ignition and exposes an SSH server on
port 22, you can `ssh` into it using podman-exec as the VMs default user:

> For this command to work with Docker, you must replace the `--latest` flag
> with the container's name or ID.

```console
$ podman run \
    --runtime crun-qemu \
    --detach --rm \
    quay.io/containerdisks/fedora:39 \
    ""
8068a2c180e0f4bf494f5e0baa37d9f13a9810f76b361c0771b73666e47ec383

$ podman exec --latest fedora whoami
fedora

$ podman exec -it --latest fedora
[fedora@8068a2c180e0 ~]$
```

With cloud-init, the default user can vary between VM images. With Ignition,
`core` is considered to be the default user. In both cases, if the SSH server
allows password authentication, you should also be able to log in as any other
user.

The `fedora` argument to podman-exec above, which would typically correspond to
the command to be executed, determines instead the name of the user to `ssh`
into the VM as. A command can optionally be specified with further arguments. If
no command is specified, a login shell is initiated. In this case, you probably
also want to pass flags `-it` to podman-exec.

If you actually just want to exec into the container in which the VM is running
(probably to debug some problem with crun-qemu itself), pass in `-` as the
username.

## Port forwarding

You can use podman-run's standard `-p`/`--publish` option to set up TCP and/or
UDP port forwarding:

```console
$ podman run \
    --runtime crun-qemu \
    --detach --rm \
    -p 8000:80 \
    quay.io/crun-qemu/example-http-server:latest \
    ""
36c8705482589cfc4336a03d3802e7699f5fb228123d18e693488ac7b80116d1

$ curl localhost:8000
<!DOCTYPE HTML>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Directory listing for /</title>
</head>
<body>
[...]
```

## Passing things through to the VM

### Directories

Bind mounting directories into the VM is supported:

> :warning: This example recursively modifies the SELinux context of all files
> under the path being mounted, in this case `./util`, which in the worst case
> **may cause you to lose access to your files**. This is due to the `:z` volume
> mount modifier, which instructs Podman to relabel the volume so that the VM
> can access it.
>
> Alternatively, you may remove this modifier from the command below and add
> `--security-opt label=disable` instead to disable SELinux enforcement.

> For this command to work with Docker, you must provide an absolute path to
> `--cloud-init`.

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    -v ./util:/home/fedora/util:z \
    quay.io/containerdisks/fedora:39 \
    --cloud-init examples/cloud-init/config
```

If the VM supports cloud-init or Ignition, the volume will automatically be
mounted at the given destination path. Otherwise, you can mount it manually with
the following command, where `<index>` must be the 0-based index of the volume
according to the order the `-v`/`--volume` or `--mount` flags where given in:

```console
$ mount -t virtiofs virtiofs-<index> /home/fedora/util
```

### Regular files

Similarly to directories, you can bind mount regular files into the VM, where
they appear as block devices:

> :warning: The warning about SELinux relabeling on the command above also
> applies here.

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    -v ./README.md:/home/fedora/README.md:z \
    quay.io/containerdisks/fedora:39 \
    --cloud-init examples/cloud-init/config
```

### Block devices

If cloud-init or Ignition are supported by the VM, it is possible to pass block
devices through to it at a specific path using podman-run's `--device` flag
(this example assumes `/dev/ram0` to exist and to be accessible by the current
user):

> For this command to work with Docker, you must provide an absolute path to
> `--cloud-init`.

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    --device /dev/ram0:/home/fedora/my-disk \
    quay.io/containerdisks/fedora:39 \
    --cloud-init examples/cloud-init/config
```

You can also pass them in as bind mounts using the `-v`/`--volume` or `--mount`
flags.

### vfio-pci devices

vfio-pci devices can be passed through to the VM by specifying the non-standard
`--vfio-pci` option with a path to the device's sysfs directory (this example
assumes that the corresponding VFIO device under `/dev/vfio/` is accessible to
the current user):

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    quay.io/containerdisks/fedora:39 \
    --vfio-pci /sys/bus/pci/devices/0000:00:01.0
```

In turn, mediated (mdev) vfio-pci devices (such as vGPUs) can be passed through
with the `--vfio-pci-mdev` option, specifying a path to the mdev's sysfs
directory:

```console
$ podman run \
    --runtime crun-qemu \
    -it --rm \
    quay.io/containerdisks/fedora:39 \
    --vfio-pci-mdev /sys/bus/pci/devices/0000:00:02.0/5fa530b9-9fdf-4cde-8eb7-af73fcdeeaae
```

[cloud-init]: https://cloud-init.io/
[Ignition]: https://coreos.github.io/ignition/
[KubeVirt `containerDisk`s]: https://kubevirt.io/user-guide/virtual_machines/disks_and_volumes/#containerdisk