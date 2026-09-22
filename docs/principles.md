# Principles

> This document describes the rules currently in force. A sentence here that is wrong is a bug.

## 1. The model is the contract

Parsers, readers, editors, differs and renderers all speak this model and nothing else to each other. A change here reaches every one of them, so changes are graded by kind:

| Change | How |
|---|---|
| **Adding** an entity type or field | Fine without prior discussion; mention it in the change description |
| **Changing the meaning of, or removing,** a field | Propose first |
| Changing the **provenance / confidence contract**, including a new confidence value | Discuss before any work |

If it is unclear which row a change falls in, treat it as the stricter one.

## 2. What every entity carries

| | Meaning |
|---|---|
| **Reference ID** | The name by which this entity is pointed at. One scheme, shared by every consumer. Minted by whoever puts the entity into the model — a parser, a recognizer, an editor — at the moment it is put there, never by a reader. Opaque to consumers, unique within a model, and the same for the same inputs. A file handle is not a reference ID: it is provenance data, carried separately, present only for entities that came from a file |
| **Provenance** | Where the entity came from — read from a vector source, recognized from a raster source, or produced by an operation. A closed set: a fourth kind is a discussion, not an addition |
| **Confidence** | How far the entity's values can be trusted. A total order — unknown, low, high — so that whichever layer merges values can take the lower one. Finer grades and numeric scores are not part of the model; where a score is folded into a grade is a product's decision, not a component's |

## 3. Invariants

1. **One reference scheme.** Consumers do not define reference or coordinate systems of their own.
2. **Confidence never rises on its own.** A value that entered with low confidence is never reported with a higher one, whatever it passes through.
3. **Nothing is silently dropped.** What a parser could not interpret stays in the model as "unrecognized".
4. **What cannot be established is "unknown".** It is a first-class value, not an error, and never replaced by a default or an estimate.
5. **Consumers need not know why confidence is low.** They act on the marker alone.

*What this costs:* every producer has to fill these markers in, and every consumer has to carry them through. There is no lightweight variant of the model without them, and none of the three has a default value — a producer states them or the entity cannot be built.

### 3.1 Three shapes of "unknown"

They are orthogonal: one entity can be recognized from a raster, of a type the model does not know, with a layer that could not be resolved, all at once.

| What is unknown | How it is carried |
|---|---|
| The entity's **kind** | An "unrecognized" entity that keeps the type name the file used |
| One **referenced value** (a layer, a block, a style) | Three states: resolved; absent, because the file carries no handle for it; unresolved, because the handle answers to nothing. The last keeps the handle — it is the only thing that tells a missing table row from references broken wholesale |
| How far the **values** can be trusted | Confidence |

An unresolved reference is never written as an empty string, in the model or in its serialization. An empty name and a name that could not be read must stay distinguishable.

## 4. Pure data, no weight

Types and serialization. No parsing, no rendering, no native code, no network. A crate that only needs to *talk about* drawings should pay for nothing else.

One piece of arithmetic lives here, and only one: the placement a block reference applies to its block -- the map from a block definition's coordinates to the drawing's, composed across nested references (`Affine2`). Every consumer that follows an INSERT needs exactly this map and needs nested references to compose the same way, or two of them disagree about where the same line is drawn; it is arithmetic on the INSERT's own fields, with nothing to guess. Distance, intersection, hit-testing, rendering and editing stay in the consumers.

The dependency tree is permissive-only (MIT / Apache-2.0 / BSD), and this crate depends on none of its consumers.

## 5. Format-shaped, not tool-shaped

The model reflects what 2D CAD drawings contain — lines, arcs, dimensions, tolerances, blocks, layers. It does not reflect the internal structures of any one parsing library, and it does not carry concepts that only one consumer needs.

The test: *would a third party using this model for the first time need the same thing in the same place?* If not, it does not belong here.

## 6. Determinism

Serialization is stable: the same model produces the same bytes. Snapshot tests depend on it.

## 7. Compatibility

The crate is in 0.x. When a more correct shape is found, a breaking change is the normal way to adopt it; it is not deferred for migration cost. Breaking changes bump the minor version. The major version is not bumped without an explicit maintainer decision.

## 8. How the model and its consumers are measured

Two layers, kept apart because they answer different questions.

| Layer | Input | Oracle | Lives in |
|---|---|---|---|
| **Golden** | Synthetic drawings written from a *spec* (Rust data) by this repository's own DXF writer, which shares no code with any parser | The spec itself: the values a reader must get back, and the model it must produce | `golden/`, a test-only crate every consumer takes as a dev-dependency |
| **Regression** | Real files a parser's own corpus provides | The distribution a previous run fixed: counts that must not change without a stated reason | The parser's repository, next to its backend |

The golden layer measures *correctness against a known truth*; the regression layer measures *change*. A golden case is one named spec (`G1`, a general part, is the first) and every role reads its share of it: the parser compares its output with the expected model, the summarizer checks its summary against the spec's values, the editor and the differ check that a change touches exactly what it was asked to.

Negative cells -- what must *never* happen: an unrequested entity changed, a low-confidence value reported high, an unknown filled with a default, a non-deterministic byte -- are not one case each but invariants over generated specs, checked by property tests with a seeded, deterministic generator that shrinks a failure to its smallest reproduction.

The layer keeps no real drawings and no parser code, so it carries no license but this crate's own.
