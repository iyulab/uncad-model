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

### 3.2 When a file leaves a value out

Writers omit groups. The DXF reference says optional groups "appear only if their values differ from the defaults", and in practice writers also omit some groups the reference does not mark optional, when their value is zero. What an omission means depends on what the group states, not on how the reference labels it:

| The group states | Omitted means | How the model carries it |
|---|---|---|
| A **departure** from a default state — a rotation away from the default direction, a ratio to the default spacing | No departure. The group's own definition fixes the value (0, or 1 for a ratio) | The plain value (`f64`). This is not a default standing in for an unknown: the file has said "no departure" in the only way its format has |
| A **fact** — whether there is an arrowhead, whether a path is curved, what kind of light it is | Nothing. The definition does not say what absence means | `Option`, and `None` when omitted. Filling in `false` or a first variant would be a claim the file never made |
| A **size** the format requires — a radius, a text height, a width | The file is malformed | The value the reader fell back on, **and** a diagnostic saying the group was missing. The type does not change |

Invariant 4 is what draws the line: a departure group's absence is established by the format, so there is nothing unknown to carry; a fact group's absence is not, so the model says so. `Option` is used where it carries information and nowhere else -- wrapping every field a writer might omit would make "unknown" the usual answer, and a consumer would learn to replace it with zero.

## 4. Pure data, no weight

Types and serialization. No parsing, no rendering, no native code, no network. A crate that only needs to *talk about* drawings should pay for nothing else.

The arithmetic that lives here is the coordinate arithmetic the format itself defines on an entity's own fields, and nothing else:

- the placement a block reference applies to its block -- the map from a block definition's coordinates to the drawing's, composed across nested references (`Affine2`, from the INSERT's 10/20, 41/42, 50, and -- when its plane is parallel to the world's, a mirror copy's included -- its 210 through `Ocs`; a plane tilted out of the world's has no exact 2D placement, and none is given);
- the coordinate system an entity is written in -- its axes from its extrusion by the arbitrary axis algorithm (`Ocs`, from 210/220/230);
- the arc a polyline vertex's bulge describes -- its center, radius and sweep from the two vertices and the bulge (`BulgeArc`, from 42).
- the point at a parameter on a curve the entity defines completely -- an ELLIPSE's, from its center, major axis, axis ratio and normal (`EllipseEntity::point_at`, from 10, 11, 40 and 210), and a NURBS curve's, from its degree, knots, control points and weights, whether a SPLINE's or a HATCH boundary's spline edge (`Nurbs`, from 71/94, 40, 10 and 41/42). A spline stored only by its fit points has no such curve here: the format does not store the curve through them.

Each is a function of fields the entity carries, with nothing to choose and nothing to guess. Every consumer that draws, measures or points at the entity needs exactly this arithmetic, and if two consumers computed it separately they could disagree about where the same line is -- which is why it is here once rather than in each of them. A tolerance this crate adds on top (when a composed placement still counts as a similarity, when an extrusion counts as the world's) is named and documented where it is defined, apart from the format's own constants.

Distance, intersection, hit-testing, curve sampling (how many points, how far a chord may stray from the curve), rendering and editing stay in the consumers: they choose something (a precision, a view, a policy for what a degenerate input means) that the format does not.

The dependency tree is permissive-only (MIT / Apache-2.0 / BSD), and this crate depends on none of its consumers.

## 5. Format-shaped, not tool-shaped

The model reflects what 2D CAD drawings contain — lines, arcs, dimensions, tolerances, blocks, layers. It does not reflect the internal structures of any one parsing library, and it does not carry concepts that only one consumer needs.

The test: *would a third party using this model for the first time need the same thing in the same place?* If not, it does not belong here.

## 6. Determinism

Serialization is stable: the same model produces the same bytes. Snapshot tests depend on it.

## 6.1 What a reader owes the model's strings

A reader fills this model from a file, and a file's text is not always Unicode. More than one reader exists, so the rule is stated once here rather than settled again in each of them:

- **Bytes that are valid UTF-8 are taken as UTF-8**, whatever the file's header declares. An ASCII file is both, and a file whose text a tool wrote as UTF-8 under a legacy header reads correctly this way.
- **Otherwise the declared code page is trusted** — no sniffing, no guessing from the content. A file that declares the wrong code page reads cleanly wrong, which is a thing its author can fix; a reader that guesses is wrong in a way nobody can predict.
- **A byte the code page has no character for becomes U+FFFD and is reported**, in the diagnostics the read returns. A code page the reader has no table for is reported the same way and the text read as UTF-8. Neither is ever silent: a string that came back wrong must be visible as such.

None of this reaches the model's types — a string field is a string. What it fixes is the one question every reader of a pre-Unicode format has to answer, answered the same way by all of them.

**How the file stored a string is undone; what the text says is not.** A file whose text is in a code page writes a character that page cannot hold as an escape, in any string — `\U+XXXX`, or `\M+nXXXX` for a character of an Asian code page — and a DXF file, whose every value is one line, writes a control character in caret notation (`^J` for a line break). That is storage: a reader turns each back into the character ([`text`](../src/text.rs) does it the same way for every reader), so a drawing saved with the character and one saved with its escape carry the same string. An escape that names an ASCII character is left as written: no writer needs one, and undoing it could turn text into a code.

What the text itself encodes is carried as written: MTEXT formatting codes and `%%` codes stay in the string. They say something about how the text is shown — underline it, stack these two parts, draw a degree sign in this font's way — and what to do with that is a consumer's choice.

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
