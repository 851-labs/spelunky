# Wine (with 32-bit support) and Xvfb on amd64 Debian, to run GM8 executables headless.
FROM --platform=linux/amd64 debian:bookworm-slim
RUN dpkg --add-architecture i386 \
 && apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      wine wine32 wine64 xvfb xauth x11-utils imagemagick ca-certificates \
 && rm -rf /var/lib/apt/lists/*
ENV WINEDEBUG=-all WINEPREFIX=/wine
RUN xvfb-run -a wineboot --init; wineserver -w; true
# Mesa's software GL, 32-bit, for the runner's Direct3D 8 (wined3d draws through OpenGL).
RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      libgl1-mesa-dri:i386 libglx-mesa0:i386 libgl1:i386 \
 && rm -rf /var/lib/apt/lists/*
