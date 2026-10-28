---
name: hypatia-dream
description: "Run an incremental Hypatia graph-consolidation pass at a work-period boundary, like sleep-time knowledge organization, using the hypatia_memory MCP tools. Use only for an explicit batch closeout or dream-time review, not for ordinary knowledge CRUD, a one-off triple, or a lookup. Without an explicit mode, closeout phrases such as 'done for the day', 'wrapping up', 'call it a night', 'end of this phase', 'sweet dreams', or 'tidy up before bed' mean apply; preview phrases such as 'dream on it first', 'review before bed', 'show me how the knowledge relates', 'report only', 'just show me first', or 'do not change anything yet' mean report. Accepts an optional shelf name and `--mode report|apply`. Statement triples are the primary evidence; Knowledge bodies only disambiguate."
---

# Hypatia Dream

Perform an incremental, relationship-focused consolidation pass over one Hypatia shelf. The
purpose is to make the graph more coherent after new knowledge arrives, not to summarize,
rewrite, delete, or otherwise curate the knowledge nodes themselves.

Treat the graph as a personal knowledge base: inspect its whole available relationship topology
rather than imposing a small relationship window. Keep the analysis disciplined by treating
Statement triples as the primary evidence. Knowledge bodies may clarify entity identity or
resolve an apparent contradiction, but they are not a license to invent weak associations.

All Hypatia reads and writes go through the `hypatia_memory` MCP tools (`list_shelves`,
`get_shelf_status`, `query`, `get_knowledge`, `create_statement`, `delete_statement`,
`create_knowledge`). See the `hypatia` skill for tool and JSE syntax. Do not shell out to a
`hypatia` binary or open shelf files directly.

## Trigger And Mode Resolution

Use this skill only for a deliberate batch consolidation at a work-period boundary or an
explicitly requested dream-time review. Do not use it for ordinary knowledge CRUD, a direct
one-off triple request, an entity lookup, a relationship query, or automatic
conversation-memory extraction; those belong to `hypatia` or `hypatia-memory`.

Take the optional shelf name from the request; use `default` when it is omitted.

Resolve the mode in this order:

1. An explicit `--mode report` or `--mode apply` always wins.
2. Without an explicit mode, treat these closeout signals and clear equivalents as `apply`:
   `done for the day`, `wrapping up`, `call it a night`, `that's it for today`,
   `end of this phase`, `phase wrap-up`, `sweet dreams`, and `tidy up before bed`.
3. Without an explicit mode, treat these preview signals and clear equivalents as `report`:
   `dream on it first`, `organize and show me`, `review before bed`,
   `show me how the knowledge relates`, `report only`, `don't modify yet`,
   `just show me first`, and `do not change anything yet`.
4. If both signal classes appear in one request, ask the user to choose a mode. Do not assume `apply`.
5. A direct but otherwise unspecified invocation defaults to `report`.

When a closeout phrase selects `apply`, say so explicitly ("'done for the day' selects apply
mode"), then follow the normal preflight, backup, verification, and watermark rules.

The modes have these effects:

- `report` reads the shelf and returns a structured report. It creates no state marker and
  changes no knowledge or statement.
- `apply` may add a missing statement or replace an explicitly contradicted statement. It
  never creates, edits, or deletes a regular Knowledge entry.

Before every `hypatia_memory` tool call, briefly state the goal, the exact tool call, why that
query or mutation is appropriate, and the expected result.

## Hypatia Conventions To Preserve

Use the actual Statement fields and tool argument names: `head`, `relation`, and `tail`. Do not
describe them as `subject`, `predicate`, or `object` in tool calls or reports.

Prefer the relationship vocabulary already established by Hypatia's semantic-memory design:

- `is_a` for a clear category relationship.
- `refines` for a more precise form of an existing concept.
- `extends` for an addition that builds on an existing concept.
- `supersedes` for an explicit replacement of an older concept or decision.
- `derivedFrom` for a clear provenance relationship.

Preserve existing relation spellings. `belongTo` and `summary` are operational relationships
maintained by the memory system; do not reinterpret, delete, replace, or add semantic links to
their message/session/summary endpoints. Likewise, do not reorganize archive metadata
relationships such as `is_a archive`.

Do not use a vague catch-all relationship such as `related_to` merely because two entries share
words or a topic. Omit uncertain candidates.

## Preflight And Snapshot Boundary

1. Confirm the `hypatia_memory` MCP tools are available. If they are missing or fail to
   connect, stop and report the blocker (the server is started with `hypatia-mcp-server serve`).
2. Set `SHELF` to the requested shelf or `default`, then confirm it is connected before doing
   anything else:

   ```json
   list_shelves
   get_shelf_status {"shelf_name": "<SHELF>"}
   ```

   If `<SHELF>` is not listed, or the status call fails, report it as a blocker. Do not connect,
   migrate, or otherwise work around it. Note any `attention` message from the status in the report.

3. Record a UTC run-start timestamp before reading graph data. This is the only shell command
   the skill needs:

   ```bash
   date -u '+%Y-%m-%d %H:%M:%S'
   ```

   Hypatia stamps `created_at` in UTC as `%Y-%m-%d %H:%M:%f`, so stored values carry
   milliseconds (`2026-09-02 08:50:08.790`) and compare as TEXT. A second-resolution boundary
   is therefore slightly conservative, deliberately so: `created_at <= "2026-09-02 08:50:08"`
   excludes `2026-09-02 08:50:08.790`, and because the watermark is written at the same
   resolution, the next run's `created_at > "2026-09-02 08:50:08"` picks that row back up. Do
   not pad the boundary with `.999` and do not truncate stored values to close the apparent
   gap — either one reintroduces a real gap.

4. Query the latest successful apply marker in the same shelf:

   ```json
   query {"jse": {"$knowledge": [["$has", "tags", "hypatia-dream-run"]], "limit": 1}, "shelf": "<SHELF>"}
   ```

   Query results are ordered by `created_at DESC`, so `limit: 1` returns the most recent
   marker. A marker is a system-tagged Knowledge entry created by this skill after a successful
   `apply`. Its `content.data` contains a `processed_through` timestamp. Read that value, not
   the marker's `created_at`: a run can create statements after its snapshot begins, and using
   the marker creation time could skip concurrent new information.

5. Define the collection interval:

   - If a valid marker exists, use `processed_through < created_at <= run_started_at`.
   - If there is no valid marker, this is the first consolidation pass. Collect all
     pre-snapshot triples and knowledge entries.
   - A malformed marker is not a reason to guess. Report it and treat the run as a first pass
     unless the user supplies an explicit baseline.

   `report` mode must not advance this watermark, so a later `apply` can reconsider the same
   proposed changes. Only a completed `apply` writes a marker.

## Gather The Working Set

Retrieve the complete pre-snapshot relationship graph. Use a large, explicit limit because the
shelf is personal and no relationship-range cap is wanted:

```json
query {"jse": {"$statement": [["$lte", "created_at", "<RUN_STARTED_AT>"]], "limit": 10000}, "shelf": "<SHELF>"}
```

Retrieve the newly created triples in the collection interval. For a first pass, drop the
`$gt` lower bound:

```json
query {"jse": {"$statement": [["$and", ["$gt", "created_at", "<BASELINE>"], ["$lte", "created_at", "<RUN_STARTED_AT>"]]], "limit": 10000}, "shelf": "<SHELF>"}
```

Also retrieve new Knowledge entries from the same interval, excluding internal log, session,
summary, and dream-marker entries by tag. For a first pass, drop the `$gt` lower bound:

```json
query {"jse": {"$knowledge": [["$and", ["$gt", "created_at", "<BASELINE>"], ["$lte", "created_at", "<RUN_STARTED_AT>"], ["$not", ["$or", ["$has", "tags", "message"], ["$has", "tags", "session"], ["$has", "tags", "summary"], ["$has", "tags", "hypatia-dream-run"]]]]], "limit": 10000}, "shelf": "<SHELF>"}
```

`limit` is a hard cap, not a promise. For each of the three queries, compare the returned
`total_count` (the number of rows returned) with the limit. If they are equal, the result was
truncated by `ORDER BY created_at DESC` and the graph in hand is incomplete. Stop, report the
truncation as a blocker, and apply nothing: a relation can only be judged contradicted,
duplicate, or missing against the whole graph, and a truncated read would let this skill
"correct" a relationship whose supporting evidence was simply cut off.

The new Knowledge set can include an unlinked node. It is eligible for one well-supported
relationship to an older graph node, but only after its body and the prospective target's body
have been read for disambiguation. Do not manufacture a relation solely to make an isolated
node connected.

For each potentially affected node, fetch its Knowledge entry with
`get_knowledge {"name": "<NAME>", "shelf": "<SHELF>"}` and inspect its one-hop incident triples
from the full graph. Read only the bodies needed to explain an entity match, source lineage,
category, refinement, extension, supersession, or direct contradiction. Never use conversation
text, session summaries, or system markers as semantic evidence.

## Reason About Relationships

Work from the newly added triples and eligible new Knowledge nodes toward the full existing
graph. For every candidate, identify:

- The exact new evidence: triple or Knowledge entry name and the relevant wording.
- The supporting old graph context: exact triples and, when needed, the old node body.
- The proposed triple in `(head, relation, tail)` form.
- A confidence level and a short explanation of why that relation is more precise than its alternatives.

Classify each outcome as exactly one of:

1. **Add**: a missing, non-duplicate triple is directly supported by the new evidence and the
   existing graph vocabulary.
2. **Replace**: a specific old triple is directly contradicted by newer, unambiguous evidence.
   Name both triples. Prefer adding `supersedes` when the old idea remains historically useful;
   delete and replace only when the old relation itself is false.
3. **No change**: evidence is duplicate, weak, ambiguous, historical rather than
   contradictory, or depends on a protected/system relation.

A candidate must be high confidence before it may be applied. Similarity, shared tags, or
lexical overlap alone are never high confidence.

Before a replacement, inspect the old statement's `content`, `tr_start`, and `tr_end`. A query
result omits NULL columns entirely, so an absent `tr_start` or `tr_end` key means the statement
has no temporal bound — not that the information is unavailable.

`create_statement` accepts only `data`, `synonyms`, `scopes`, and `embed` as content. It cannot
set `tags`, `format`, or `figures`, and cannot recreate a temporal bound. Report instead of
replacing whenever the source statement has any of these:

- a populated `tr_start` or `tr_end`;
- a non-empty `content.tags`;
- a `content.format` other than `markdown`;
- a non-empty `content.figures`.

Otherwise the statement is eligible, and the replacement must carry over `content.data`,
positional `content.synonyms`, `content.scopes`, and `content.embed` (when present) from the
source.

## Apply Changes

In `report` mode, stop after producing the report. Do not write a backup or a marker.

In `apply` mode, execute this sequence only after the analysis identifies high-confidence changes:

1. **Re-check.** Re-query every proposed old and new triple immediately before mutating, using
   the exact-key form, which matches on the `triple` primary key:

   ```json
   query {"jse": {"$statement": [["$triple", "<HEAD>", "<RELATION>", "<TAIL>"]], "limit": 1}, "shelf": "<SHELF>"}
   ```

   Skip a stale plan, an addition that already exists, and a replacement whose source triple
   no longer matches the reviewed evidence.

   A replacement whose new triple already exists needs one more check, because
   `create_statement` never overwrites an existing triple. Compare that triple's `data`,
   `synonyms`, and `scopes` with the source triple's. Treat an absent field and an empty list or
   object as equal, and compare `scopes` as a set.

   - If they are equal, an earlier run created it and stopped before the delete. Mark the
     replacement **delete-only** and count it as replaced. Do not delete anything yet: step 4
     runs the delete after step 2's backup.
   - If they differ, skip the replacement. List it under No Change with the reason
     `target already exists with different metadata; a person must decide`, do not delete the
     source, and leave both triples out of step 5's verification.

2. **Back up.** The MCP server has no shelf-export tool, so take a statement-level backup
   before the first replacement step. Write the full JSON rows — exactly as returned by the
   step 1 re-query — of every source triple the run will delete to
   `~/.hypatia/backups/hypatia-dream-<SHELF>-<UTC_COMPACT_TIMESTAMP>.json`, creating the
   directory if needed, and report that path. This is enough to recreate every deleted triple
   with its metadata. If the file cannot be written (for example in a sandbox), include the
   rows verbatim in the report's Backup section instead, and say so. Never delete a triple
   whose row has not been captured by one of these two means.

3. **Add.** For an addition, create only the exact reviewed triple. Do not create supporting
   nodes or generic fallback links.

   ```json
   create_statement {"head": "<HEAD>", "relation": "<RELATION>", "tail": "<TAIL>", "shelf": "<SHELF>"}
   ```

   `create_statement` is idempotent. If the triple already exists, the result has
   `"created": false` and the stored triple and its metadata are left unchanged. That means the
   relationship is already recorded: count it as skipped and continue. It is not a run failure.

4. **Replace.** For a replacement, **create the new triple first, verify it, and only then
   delete the source triple.** A replacement changes at least one of `head`, `relation`, and
   `tail`, so the two triples have different primary keys and can coexist in between. This
   ordering has no window in which the old relationship is gone and the new one does not yet
   exist. Never delete first. For a delete-only replacement from step 1, skip the create and
   run only the `delete_statement`.

   ```json
   create_statement {"head": "<NEW_HEAD>", "relation": "<NEW_RELATION>", "tail": "<NEW_TAIL>", "data": "<SOURCE_DATA>", "synonyms": {"head": [], "relation": [], "tail": []}, "scopes": ["<SOURCE_SCOPE>"], "shelf": "<SHELF>"}
   delete_statement {"head": "<OLD_HEAD>", "relation": "<OLD_RELATION>", "tail": "<OLD_TAIL>", "shelf": "<SHELF>"}
   ```

   Carry the source metadata across exactly, and omit any argument whose source value is empty
   or absent. `synonyms` is positional: copy the source's `{"head": [...], "relation": [...],
   "tail": [...]}` object as is. `scopes` is copied as the same list, including any `""`
   element (the global scope): source scopes `["project-a", ""]` are passed as
   `["project-a", ""]`. If the source content has `"embed": false`, pass `"embed": false`.

   If the create fails, nothing has been lost: report it and leave the source triple untouched.
   If the create returns `"created": false`, the new triple appeared after step 1's re-query and
   may not carry the source metadata. Do not delete the source yet: apply step 1's
   already-exists check to it now. If the create succeeds but the delete fails, stop and report
   both triples as present — the graph is consistent but now carries a duplicate relationship,
   and a person must decide.

5. **Verify.** Re-query each created triple and every intended removal with the same `$triple`
   form. For a delete-only replacement, also re-query its target: it must still come back. A
   created triple must come back; a removed triple must return empty `rows`. `delete_statement`
   fails with a not-found error when the triple is already gone; that is an already-applied
   removal rather than a failure, but confirm it by query before continuing. If any
   verification fails, stop, report the partial outcome and the backup location, and do not
   create a watermark.

After all intended changes verify, write one run marker in the same shelf:

```json
create_knowledge {"name": "hypatia-dream-run-<UTC_COMPACT_TIMESTAMP>", "data": "schema: 1\nmode: apply\nprocessed_through: <RUN_STARTED_AT>\nadded: <COUNT>\nreplaced: <COUNT>\nreport: Hypatia Dream completed successfully", "tags": ["system", "hypatia-dream-run"], "embed": false, "shelf": "<SHELF>"}
```

The marker is operational metadata, not semantic knowledge. Never attach a Statement triple to
it or use it as relationship evidence. `"embed": false` keeps it out of `similar` results, but
it is still full-text indexed and will surface in the user's own `search` results, so keep the
body to the fields above — never store report prose, analysis, or candidate lists in it. This
skill never deletes an old marker; only the most recent one is ever read.

If no high-confidence graph changes exist, `apply` may still write the marker after reporting a
verified no-op. This records that the interval has been reviewed without falsely claiming a
graph mutation.

## Required Report

Return the report in this structure:

```markdown
# Hypatia Dream Report

## Scope
- Shelf: `<name>`
- Mode: `report` or `apply` (and the signal that selected it)
- Baseline: first pass, or `<processed_through>`
- Snapshot boundary: `<run_started_at>`
- Reviewed: `<new knowledge count>` Knowledge entries and `<new statement count>` new triples against `<total statement count>` existing triples
- Graph read: `<returned row count>` of `<limit>` rows, complete or truncated

## Relationship Findings
### Add
- `(head, relation, tail)` [high confidence]
  Evidence: ...
  Existing context: ...

### Replace
- `(old head, old relation, old tail)` -> `(new head, new relation, new tail)` [high confidence]
  Evidence: ...
  Rationale: ...

### No Change
- Item: ...
  Reason: duplicate, insufficient evidence, protected system relation, metadata/temporal safety restriction, or target already exists with different metadata.

## Outcome
- Planned or applied additions: `<count>`
- Planned or applied replacements: `<count>`
- Skipped candidates: `<count>`
- Backup: `<path>`, `inline below`, or `not needed`
- Watermark: `<processed_through>` written only after a verified `apply`, otherwise `unchanged`
```

Be concise but include every tool call that changed state and every exact triple it affected.
Do not claim a relationship was corrected unless its post-mutation query verified the result.
