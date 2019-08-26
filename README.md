cport: cmake container builder
===============================

Build cmake project in a container

How to use
-----------

1. Put configure TOML `cport.toml` on the root directory of cmake

```toml
[cport]
image = "debian"
apt   = ["cmake", "g++", "ninja-build", "libboost-dev"]

[cmake]
generator = "Ninja"
build     = "_cport"

[cmake.option]
CMAKE_EXPORT_COMPILE_COMMANDS = "ON"
```

2. Create container, and install dependents (speficied by `apt` in the TOML)

```
cport install
```

3. Build!

```
cport build
```
