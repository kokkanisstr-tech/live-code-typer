---------------- MODULE TypingCore ----------------
EXTENDS Integers, Sequences, FiniteSets

Alphabet == {"Letter", "Space", "Newline"}
RECURSIVE Seqs(_)
Seqs(n) ==
    IF n = 0
    THEN {<<>>}
    ELSE {Append(prefix, item) : prefix \in Seqs(n - 1), item \in Alphabet}

Codes == UNION {Seqs(length) : length \in 0..4}

ASSUME Codes \subseteq Seq(Alphabet)

VARIABLES code, cursor, typing, locked, lockedCode, output
vars == <<code, cursor, typing, locked, lockedCode, output>>

Init ==
    /\ code \in Codes
    /\ cursor = 0
    /\ typing = FALSE
    /\ locked = FALSE
    /\ lockedCode = code
    /\ output = <<>>

Start ==
    /\ ~typing
    /\ cursor < Len(code)
    /\ typing' = TRUE
    /\ UNCHANGED <<code, cursor, locked, lockedCode, output>>

Pause ==
    /\ typing
    /\ typing' = FALSE
    /\ UNCHANGED <<code, cursor, locked, lockedCode, output>>

Step ==
    /\ typing
    /\ cursor < Len(code)
    /\ cursor' = cursor + 1
    /\ output' = Append(output, code[cursor + 1])
    /\ typing' = (cursor + 1 < Len(code))
    /\ UNCHANGED <<code, locked, lockedCode>>

Restart ==
    /\ cursor' = 0
    /\ output' = <<>>
    /\ typing' = FALSE
    /\ UNCHANGED <<code, locked, lockedCode>>

Edit ==
    /\ ~locked
    /\ code' \in Codes
    /\ cursor' = 0
    /\ output' = <<>>
    /\ typing' = FALSE
    /\ UNCHANGED <<locked, lockedCode>>

Pin ==
    /\ ~locked
    /\ locked' = TRUE
    /\ lockedCode' = code
    /\ UNCHANGED <<code, cursor, typing, output>>

Unpin ==
    /\ locked
    /\ locked' = FALSE
    /\ UNCHANGED <<code, cursor, typing, lockedCode, output>>

Next == Start \/ Pause \/ Step \/ Restart \/ Edit \/ Pin \/ Unpin
Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ code \in Codes
    /\ cursor \in 0..Len(code)
    /\ typing \in BOOLEAN
    /\ locked \in BOOLEAN
    /\ lockedCode \in Codes
    /\ output \in Seq(Alphabet)

CursorBound == cursor <= Len(code)
OutputMatchesCursor == output = SubSeq(code, 1, cursor)
TypingHasNext == typing => cursor < Len(code)
PinnedContentDoesNotChange == locked => code = lockedCode

============================================================
