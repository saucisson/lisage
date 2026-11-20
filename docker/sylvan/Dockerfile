FROM ubuntu:24.04

RUN apt-get update -qq && apt-get install -y -qq \
    build-essential \
    git \
    ca-certificates \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/sylvan

# Pin by commit SHA (tag v1.12.8 = fe8f9463a5f64ea8542534d82dd74a6766de127b).
# Sylvan fetches its Lace dependency via CMake FetchContent (pinned by Sylvan
# itself at v1.11.1), so the build needs git and network access.
RUN git clone --depth 1 https://github.com/trolando/sylvan.git . \
    && git fetch --depth 1 origin fe8f9463a5f64ea8542534d82dd74a6766de127b \
    && git checkout fe8f9463a5f64ea8542534d82dd74a6766de127b \
    && cmake -B build -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF . \
    && cmake --build build --parallel "$(nproc)"

# Full measurement witness for the local confrontation (outside CI,
# ADR-0102). Reads on stdin: n (number of variables) then 2^n values 0/1
# (truth table). Writes on stdout one JSON line of the confrontation
# protocol (outil, variante, charge, parametres, temps_ms, rss_max_ko,
# noeuds, verifie).
COPY sylvan_measure.c /opt/sylvan_measure.c
RUN LACE_INCLUDE="$(find build/_deps -maxdepth 3 -type d -name src -path '*lace*' | head -1)" \
    && LACE_LIB="$(find build -name 'liblace.a' | head -1)" \
    && gcc -O2 /opt/sylvan_measure.c -o /usr/local/bin/sylvan_measure \
        -I/opt/sylvan/src -I/opt/sylvan/build/src -I"$LACE_INCLUDE" -I"$(dirname "$LACE_LIB")" \
        /opt/sylvan/build/src/libsylvan.a "$LACE_LIB" \
        -lpthread -lm \
    && printf '2\n0 1 1 0\n' | sylvan_measure

CMD ["/bin/bash"]
