FROM ubuntu:24.04

RUN apt-get update -qq && apt-get install -y -qq \
    build-essential \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/cudd

# Pin by commit SHA (tag cudd-3.0.0 = fc30c18f8db2e7d03d13241038b61eb0056a734e)
RUN git clone --depth 1 https://github.com/ivmai/cudd.git . \
    && git fetch --depth 1 origin fc30c18f8db2e7d03d13241038b61eb0056a734e \
    && git checkout fc30c18f8db2e7d03d13241038b61eb0056a734e

RUN ./configure --enable-shared --enable-obj --disable-dependency-tracking \
    && make -j"$(nproc)" \
    && make install \
    && ldconfig

# Binaire de comptage CUDD pour le différentiel local (hors CI).
# Lit sur stdin : n (nombre de variables) puis 2^n valeurs 0/1 (table de vérité).
# Écrit sur stdout : le nombre de nœuds du BDD CUDD (un entier suivi d'un retour à la ligne).
COPY cudd_count.c /opt/cudd_count.c
RUN gcc -O2 /opt/cudd_count.c -o /usr/local/bin/cudd_count \
        -I/usr/local/include -L/usr/local/lib -lcudd \
    && printf '2\n0 1 1 0\n' | cudd_count

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
