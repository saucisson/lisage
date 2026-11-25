FROM ubuntu:24.04

RUN apt-get update -qq && apt-get install -y -qq \
    build-essential \
    git \
    ca-certificates \
    autoconf \
    automake \
    libtool \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/libddd

# Pin by commit SHA (lip6/libddd main @ b775aed3e8405a8906b10f937f6dbcc4a4b8831c).
# The git checkout has no generated configure script: autoreconf first.
RUN git clone --depth 1 https://github.com/lip6/libddd.git . \
    && git fetch --depth 1 origin b775aed3e8405a8906b10f937f6dbcc4a4b8831c \
    && git checkout b775aed3e8405a8906b10f937f6dbcc4a4b8831c \
    && autoreconf -fi

RUN ./configure --disable-shared --enable-static --disable-dependency-tracking \
    && make -j"$(nproc)" \
    && make install \
    && ldconfig

# Full measurement witness for the local confrontation (outside CI,
# ADR-0102). Reads on stdin: n (number of variables) then 2^n values 0/1
# (truth table). Writes on stdout one JSON line of the confrontation
# protocol (outil, variante, charge, parametres, temps_ms, rss_max_ko,
# noeuds, verifie).
COPY libddd_measure.cpp /opt/libddd_measure.cpp
RUN g++ -O2 /opt/libddd_measure.cpp -o /usr/local/bin/libddd_measure \
        -I/usr/local/include -L/usr/local/lib -lDDD \
    && printf '2\n0 1 1 0\n' | libddd_measure

CMD ["/bin/bash"]
