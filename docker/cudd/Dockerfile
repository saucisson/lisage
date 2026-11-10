FROM ubuntu:24.04

RUN apt-get update -qq && apt-get install -y -qq \
    build-essential \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/cudd

RUN git clone --depth 1 --branch cudd-3.0.0 https://github.com/ivmai/cudd.git .

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

CMD ["/bin/bash"]
