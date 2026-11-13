# Second golden thread example: the sliding window protocol

*The most verified protocol in software engineering, and its internal mechanics is the Lisage mechanics: the window slides (the shift), the sequence base is the argument on the edge, the ISN is eliminated by the canonical section, the settled past and the unsent future are two absorbed runs. The philosophers were the showcase of the pool; this one is the showcase of the controller. Verified by simulation on November 10, 2026: exhaustive BFS against the language, exhaustive verdicts on all words, bottom-up construction from the horizon, counting fold by three lanes, closure by events, trace of the moving anchor.*

---

## 0. The result in one table

| SWP representation (sender side, SACK) | Nodes | What must be decided |
|---|---|---|
| Expallowed enumeration | infinite: the base is unbounded; 2^{k+1} − 1 states per base, any base | nothing to decide: impossible |
| Flat MDD, base bounded to B | also constant (the predicate is trivial there) | the bound B, the ISN, the window k: each triple is one more compiled model, and the slide moves the base to the bottom of the domain |
| **Lisage folded + composed anchor** | **4 nodes (EN_VOL, LIBRE, ⊤, ⊥), for any base, any window k, any ISN** | **nothing: no bounded domain, no recompilation** |

The slogan of the second example: **safety is a typing, infinity is the default, the window slides because the anchor slides.**

---

## 1. The level-1 model (the language)

```
type Cell = Tag { settled, in-flight, acknowledged, free }   // the state of a sequence number
          // in-flight : sent, not acknowledged (in flight) ; acknowledged : sent, the receiver has it
          // settled : delivered and out of the window ; free : never sent

send   : the smallest free seq < horizon → in-flight     // the send frontier advances
ack  : ∀i in the window, seq i = in-flight → acknowledged   // SACK : any position
slide  : seq base = acknowledged → base +1                 // the base advances by one step

let init = (ISN, all free)                            // an edge (D-abs), not a tree
```

The sequence space is unbounded in both useful directions: the base grows without limit, the unsent future is infinite. The sender's window is k; acknowledgments are selective (any in-flight message can be acknowledged).

---

## 2. The controller C_swp

```
C_swp = ( δ           : Tag{settled, in-flight, acknowledged, free}
        , Σ            : the sequence space ; the reading materializes only a segment of it
        , decomposition + O   : positional : cell i before cell i+1, from the base
        , R            : R-collapse (the absolute collapses into ⊤, verified), R-share, D-abs (the two runs)
        , defaults     : two, directional : behind the base, settled ; ahead of the send
                        frontier, free. The default is not a key : it is a direction.
        , D^def        : free, absorbable on any suffix
        , encoding     : relative everywhere : numbers relative to the base ; the ISN is eliminated
                        by the canonical section (min = 0)
        , attribution  : inherited = (beyond, shift), beyond = "the previous cell is ahead of the
                        send frontier" ; synthesized = (ℕ,+,×) ;
                        memory = cache (event, form, composed anchor)
        , anchor       : composed {base : b, horizon : k}
        )
```

---

## 3. The reachable language

> **SWP = ℕ × {settled^b (in-flight|acknowledged)^j free^∞ : j ≤ k}**, and the reachable is exactly the well-typed.

Verified by exhaustive BFS (k = 1..5, base ≤ 4): the reachable set equals the well-formed language, with exactly 2^{k+1} − 1 window configurations per base (3, 7, 15, 31, 63: verified). Translation invariance is total: the reachable is a product, no sequence position is special, the base is a free ray. This is the hidden theorem of the word "sliding": the same state relative to any base.

---

## 4. The representation: 2 forms, 2 terminals, 2 absorbed runs, 1 composed anchor

```
window head : hash-equal to EN_VOL (C1 merges the input into the first node :
                  no seam, the keys in-flight, acknowledged, free are all placed there)
  EN_VOL : in-flight → EN_VOL   acknowledged → EN_VOL   free → LIBRE    // in the sent prefix
  LIBRE  : free → LIBRE                                     // the loop ; in-flight, acknowledged : not placed
  at the horizon (shift = k) : EN_VOL → ⊤, LIBRE → ⊤             // the two legal ends
```

Reading of the mechanisms:

- **Handle**: (C_swp, form, {base : b, horizon : k}). The anchor is composed: two named shifts, one per fragment (the base, the horizon).
- **The settled run: the canonical instance of M4-2**. The input edge carries the argument b, the length of the run: the first window node is at position b, the minimum of the non-settled keys, the canonical section min = 0. The normalization that spec § 2.4.5 stated as a canonical instance has here its living model.
- **The free run: the future is the default**. The LIBRE loop is absorbed: a run of any length, including infinite, reads as the single edge. The infinite sequence never exists materially; open question 10 (the coinductive) is not open for all that: here infinity is the default, not an object.
- **The ISN and Wireshark**. TCP draws its ISN at random (0 to 2³²−1 during the SYN); Wireshark by default converts all numbers into relative numbers starting at 0 (official wiki). The most widespread tool on the planet performs the canonical section min = 0 by default, for readability. Lisage gives the theory and exploits it: the pool is indein-flightent of the ISN, the ISN lives on the input edge.
- **Minimality**: 2 forms, neither more nor less: the minimal automaton of (in-flight|acknowledged)*free*. EN_VOL ≠ LIBRE: the suffix [in-flight] (length 1) distinguishes them. The head merges with EN_VOL: the settled behind the base and the in-flight in the window are the same inherited, there is no seam promise (the counterpoint of the philosophers, where S was irreducible).
- **The pool**: 4 nodes (2 forms + 2 terminals), for any base, any window, any ISN. And the **absolute** DD collapses entirely into ⊤ (verified): the reachability predicate is trivial over the typed domain. All the content of the protocol lives in the controller: typing, anchors, arguments. The philosophers showed a rich pool and a simple controller; here the exact opposite.

---

## 5. Events as transductions

| Transduction | Width | Lisage mechanics |
|---|---|---|
| send | 1 cell | the send frontier advances: a single class, not k: the guard reads on the inherited (beyond = 0), the position does not exist |
| ack(i) | 1 cell | k classes: the descent follows the shift i; the backbone stays EN_VOL (in-flight and acknowledged are the same form): the image of the path is the same handle |
| slide | 1 cell + the anchor | **the anchor shift**: {base : b} → {base : b+1}; the pool does not move |

Verified trace (k = 4): the anchor moves, the pool never.

```
init       base = 0, window LLLL   anchor {base: 0, horizon: 4}
send      base = 0, window PLLL   {base: 0, horizon: 4}
send      base = 0, window PPLL   {base: 0, horizon: 4}
ack(0)  base = 0, window APLL   {base: 0, horizon: 4}
slide     base = 1, window PLLL   {base: 1, horizon: 4}   // the argument increments
ack(0)  base = 1, window ALLL   {base: 1, horizon: 4}
slide     base = 2, window LLLL   {base: 2, horizon: 4}   // the pool did not move once
```

The most banal event of the protocol (the base advances) is, in the Lisage vocabulary, an argument increment. The very name of the protocol is the mechanism: the window slides because the anchor slides.

---

## 6. Saturation: zero rounds

The well-typed set is already closed under the three events: verified exhaustively (k = 1..5, base ≤ 3: 764 firings at k = 5, zero exits from the language, zero blocking). The fixpoint is reached without exploration: **the reachability proof is a typing, not a saturation**. The philosophers discovered their forms in two to three rounds; here the controller already knew everything, discovery only pays for what the typing does not guarantee. The deadlock is empty (verified: send armed if j < k, ack armed on any in-flight, slide armed on any acknowledged base).

---

## 7. Derived operations

- **count** (synthesized, (ℕ,+,×)) : 2^{k+1} − 1 per base (verified by three lanes up to k = 24 : 2²⁵ − 1 = 33 554 431 ; by BFS up to k = 5). The fold : n_E(ℓ) = 2n_E(ℓ−1) + n_L(ℓ−1), n_L(ℓ) = n_L(ℓ−1), the matrix [[2,1],[0,1]] : the simplest in the bestiary, eigenvalues 2 and 1. Growth 2 per cell, the simplest of growths, carried by the simplest of forms.
- **member** (inherited) : O(k) on the expallowed window, O(1) on the two absorbed runs: the length of the run is compared to the argument, not traversed.
- **exists** : "is the window full?": the inherited at the last window level, one reading, no unfolding.
- **deadlock** : empty (above).

---

## 8. What the example proves, and what it does not prove

**Verified (simulation, November 10, 2026)**: the language (BFS == well-typed == DD verdict, exhaustive over the 3^k window words for k ≤ 8); the 4 nodes (bottom-up construction from the horizon: the 2 forms at the first level, zero creation at each following level, the head merges with EN_VOL, the same pool for k = 1..50); the fold (three lanes); the closure (zero exit, zero blocking); the trace of the moving anchor.

**What it shows that the philosophers did not show**: (1) the **composed anchor** {base, horizon}; (2) the **living argument**: the base moves (slide), the ISN varies (canonical section): the canonical instance of M4-2 in real conditions, with an industrial precedent (Wireshark); (3) **two absorbed runs**, one of them infinite, without opening the coinductive; (4) the **typing relative to the argument** (in-flight, acknowledged not placed ahead of the send frontier: clause iv of the algebra contract of spec § 2.4.5); (5) a **controller that carries everything**: the pool is the smallest possible, safety reads on the declared keys.

**Not proven, planned**: the receiver side and the channels (loss, duplication, reordering: the full SWP). There the content becomes non-trivial again and the pool becomes necessary again: this is the ground of dynamic proof, the logical continuation of this example. The measured comparison against an existing encoder remains to be done; the SWP has been verified by PVS, by μCRL, by IOA and MONA (Springer links repatriated), and the regular model checking lineage (survey Abdulla, Jonsson et al., STTT) treats infinite families via rational sets: Lisage is the DD side of the same intuition, to be brought back into the state of the art for positioning.

**Roles in the portfolio**: the philosophers = the showcase of the pool (sharing, symbolic anchor, seam, counting); the window = the showcase of the controller (composed anchors, arguments, typing, runs). The first says "the pool pays 1 instead of 4n"; the second says "the controller already knows everything, the pool discovers nothing". Rennes sprint: an example outside the Petri family, a recognized network engineering object, and the names of the protocol (seq, ack, window, ISN) are industry vocabulary: the semantic reading of petrichor reads the symmetry hypothesis there as a theorem (translation invariance, § 3), the "shuttle" track of the backlog has its school case.
