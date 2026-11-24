FROM ubuntu:24.04

RUN apt-get update -qq && apt-get install -y -qq \
    build-essential \
    autoconf \
    automake \
    libtool \
    m4 \
    perl \
    git \
    ca-certificates \
    autoconf \
    automake \
    libtool \
    perl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/cudd

# Pin by commit SHA (tag cudd-3.0.0 = fc30c18f8db2e7d03d13241038b61eb0056a734e)
# The git checkout has no generated configure script: autoreconf first.
RUN git clone --depth 1 https://github.com/ivmai/cudd.git . \
    && git fetch --depth 1 origin fc30c18f8db2e7d03d13241038b61eb0056a734e \
    && git checkout fc30c18f8db2e7d03d13241038b61eb0056a734e \
    && autoreconf -fi

RUN ./configure --enable-shared --enable-obj --disable-dependency-tracking \
    && make -j"$(nproc)" \
    && make install \
    && ldconfig

# CUDD counting binary for the local differential (outside CI).
# Reads from stdin: n (number of variables) then 2^n 0/1 values (truth table).
# Writes to stdout: the number of nodes of the CUDD BDD (an integer followed by a newline).
COPY cudd_count.c /opt/cudd_count.c
COPY cudd_measure.c /opt/cudd_measure.c
RUN gcc -O2 /opt/cudd_count.c -o /usr/local/bin/cudd_count \
        -I/usr/local/include -L/usr/local/lib -lcudd \
    && gcc -O2 /opt/cudd_measure.c -o /usr/local/bin/cudd_measure \
        -I/usr/local/include -L/usr/local/lib -lcudd \
    && printf '2\n0 1 1 0\n' | cudd_count \
    && printf '2\n0 1 1 0\n' | cudd_measure

# Binaire de confrontation (ADR-0102, protocole dans
# docs/bench-confrontation-temoins-et-mesures.md) : une charge, une
# mesure, une ligne JSON sur stdout. Fumée : alea-lcg 4 (vérification
# interne : 7 satisfactions) et fenetre 4 10 (40 satisfactions).
COPY cudd_bench.c /opt/cudd_bench.c
RUN gcc -O2 /opt/cudd_bench.c -o /usr/local/bin/cudd_bench \
        -I/usr/local/include -L/usr/local/lib -lcudd -lm \
    && cudd_bench alea-lcg 4 \
    && cudd_bench fenetre 4 10

CMD ["/bin/bash"]
