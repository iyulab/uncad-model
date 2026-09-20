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
| **Reference ID** | The name by which this entity is pointed at. One scheme, shared by every consumer |
| **Provenance** | Where the entity came from — read from a vector source, recognized from a raster source, or produced by an operation |
| **Confidence** | How far the entity's values can be trusted. Includes "unknown" |

## 3. Invariants

1. **One reference scheme.** Consumers do not define reference or coordinate systems of their own.
2. **Confidence never rises on its own.** A value that entered with low confidence is never reported with a higher one, whatever it passes through.
3. **Nothing is silently dropped.** What a parser could not interpret stays in the model as "unrecognized".
4. **What cannot be established is "unknown".** It is a first-class value, not an error, and never replaced by a default or an estimate.
5. **Consumers need not know why confidence is low.** They act on the marker alone.

*What this costs:* every producer has to fill these markers in, and every consumer has to carry them through. There is no lightweight variant of the model without them.

## 4. Pure data, no weight

Types and serialization only. No parsing, no rendering, no geometry algorithms, no native code, no network. A crate that only needs to *talk about* drawings should pay for nothing else.

The dependency tree is permissive-only (MIT / Apache-2.0 / BSD), and this crate depends on none of its consumers.

## 5. Format-shaped, not tool-shaped

The model reflects what 2D CAD drawings contain — lines, arcs, dimensions, tolerances, blocks, layers. It does not reflect the internal structures of any one parsing library, and it does not carry concepts that only one consumer needs.

The test: *would a third party using this model for the first time need the same thing in the same place?* If not, it does not belong here.

## 6. Determinism

Serialization is stable: the same model produces the same bytes. Snapshot tests depend on it.

## 7. Compatibility

The crate is in 0.x. When a more correct shape is found, a breaking change is the normal way to adopt it; it is not deferred for migration cost. Breaking changes bump the minor version. The major version is not bumped without an explicit maintainer decision.
