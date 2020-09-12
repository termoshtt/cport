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

License
--------

Copyright 2019-2020 Toshiki Teramura <toshiki.teramura@gmail.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
