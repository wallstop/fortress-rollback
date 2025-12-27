       Summary of TLA+

           Module-Level Constructs

           The Constant Operators

           Miscellaneous Constructs

           Action Operators


           Temporal Operators
-

           User-Definable Operator Symbols

           Precedence Ranges of Operators

           Operators Defined in Standard Modules.

           ASCII Representation of Typeset Symbols
   Module-Level Constructs

                   module M
   Begins the module or submodule named M .

   extends M 1, . . . , M n

   Incorporates the declarations, definitions, assumptions, and theorems from

   the modules named M 1, . . . , M n into the current module.

-  constants C 1, . . . , C n (1)

   Declares the C j to be constant parameters (rigid variables). Each C j is either
   an identifier or has the form C ( , . . . , ), the latter form indicating that C

   is an operator with the indicated number of arguments.

   variables x 1, . . . , x n (1)
      Declares the x j to be variables (parameters that are flexible variables).

   assume P
      Asserts P as an assumption.

   F (x 1, . . . , x n ) = exp
      Defines F to be the operator such that F (e1, . . . , en ) equals exp with each
      identifier x k replaced by ek . (For n = 0, it is written F = exp.)

   f [x  S ] = exp (2)
      Defines f to be the function with domain S such that f [x ] = exp for all x
      in S . (The symbol f may occur in exp, allowing a recursive definition.)

   (1) The terminal s in the keyword is optional.
   (2) x  S may be replaced by a comma-separated list of items v  S , where v is either a

        comma-separated list or a tuple of identifiers.

   instance M with p1  e1, . . . , pm  em
      For each defined operator F of module M , this defines F to be the operator
      whose definition is obtained from the definition of F in M by replacing each
      declared constant or variable pj of M with ej . (If m = 0, the with is omitted.)
   N (x 1, . . . , x n ) = instance M with p1  e1, . . . , pm  em

      For each defined operator F of module M , this defines N (d 1, . . . , d n )!F to be
      the operator whose definition is obtained from the definition of F by replacing
      each declared constant or variable pj of M with ej , and then replacing each
      identifier x k with d k . (If m = 0, the with is omitted.)

           theorem P
              Asserts that P can be proved from the definitions and assumptions of the
              current module.



-  local def

   Makes the definition(s) of def (which may be a definition or an instance

   statement) local to the current module, thereby not obtained when extending

   or instantiating the module.

   Ends the current module or submodule.
   The Constant Operators

   Logic
      ¬
      true false boolean [the set {true, false}]
       x  S : p (1)  x  S : p (1)
      choose x  S : p [An x in S satisfying p]

   Sets

   = =  /    \ [set difference]

-  {e1, . . . , en }   [Set consisting of elements ei ]

   {x  S : p} (2) [Set of elements x in S satisfying p]

   {e : x  S } (1) [Set of elements e such that x in S ]

   subset S            [Set of subsets of S ]

   union S             [Union of all elements of S ]

   Functions                         [Function application]
      f [e]                          [Domain of function f ]
      domain f                       [Function f such that f [x ] = e for x  S ]
      [x  S  e] (1)                  [Set of functions f with f [x ]  T for x  S ]
      [S  T ]                        [Function f equal to f except f [e1] = e2]
      [f except ![e1] = e2] (3)

   Records                           [The h-field of record e]
      e .h                           [The record whose hi field is ei ]
      [h1  e1, . . . , hn  en ]      [Set of all records with hi field in S i ]
      [h1 : S 1, . . . , hn : S n ]  [Record r equal to r except r .h = e]
      [r except !.h = e] (3)

   Tuples              [The i th component of tuple e]
      e[i ]            [The n-tuple whose i th component is ei ]
       e1, . . . , en  [The set of all n-tuples with i th component in S i ]
      S1 × ... × Sn

   (1) x  S may be replaced by a comma-separated list of items v  S , where v is either a
        comma-separated list or a tuple of identifiers.

   (2) x may be an identifier or tuple of identifiers.

   (3) ![e1] or !.h may be replaced by a comma separated list of items !a1 · · · an , where each
        ai is [ei ] or .hi .
   Miscellaneous Constructs

   if p then e1 else e2                       [e1 if p true, else e2]
   case p1  e1 2 . . . 2 pn  en               [Some ei such that pi true]
   case p1  e1 2 . . . 2 pn  en 2 other  e    [Some ei such that pi true,
                                              or e if all pi are false]

   let d 1 = e1 . . . d n = en in e [e in the context of the definitions]

    p1 [the conjunction p1  . . .  pn ]   p1 [the disjunction p1  . . .  pn ]

-  ...                                   ...

    pn                                    pn

   Action Operators

   e            [The value of e in the final state of a step]
   [A]e         [A  (e = e)]
    Ae          [A  (e = e)]
   enabled A    [An A step is possible]
   unchanged e  [e = e]
   A·B          [Composition of actions]

   Temporal Operators

                2F   [F is always true]

                3F   [F is eventually true]

                WFe (A) [Weak fairness for action A]

                SFe (A) [Strong fairness for action A]
                F ; G [F leads to G]
   User-Definable Operator Symbols

   Infix Operators

   + (1)  - (1)      (1)     / (2)     (3)                    ++

   ÷ (1)  % (1)     ^ (1,4)  . . (1)  ...                     --

    (5)      (5)                                              

   < (1)  > (1)      (1)      (1)                             //

                                                              ^^

                    <:       : >(6)   &                       &&

   <      =             (5)           |                       %%

-                                                             @@ (6)

                    |=       =|       ·                       ##

                             =        $                       $$
                             =.
          ::=                         ??                      !!

   

   Postfix Operators (7)

      ^+ ^ ^#

   (1) Defined by the Naturals, Integers, and Reals modules.
   (2) Defined by the Reals module.
   (3) Defined by the Sequences module.
   (4) x ^y is printed as x y .
   (5) Defined by the Bags module.
   (6) Defined by the TLC module.
   (7) e^+ is printed as e+, and similarly for ^ and ^#.
   Precedence Ranges of Operators

   The relative precedence of two operators is unspecified if their ranges overlap.
   Left-associative operators are indicated by (a).

                                    Prefix Operators

             ¬    4­4               2  4­15           union 8­8

        enabled 4­15                3  4­15 domain 9­9

        unchanged 4­15 subset 8­8                         -  12­12

-

                                    Infix Operators

    1­1           5­5                  < : 7­7                       11­11 (a)
                        5­5             \ 8­8                 - 11­11 (a)
   -+ 2­2                                8­8 (a)             -- 11­11 (a)
                  5­5                    8­8 (a)              & 13­13 (a)
    2­2                 5­5             . . 9­9              && 13­13 (a)
                                       . . . 9­9
   ; 2­2          5­5                   ! ! 9­13                     13­13 (a)
                  5­5                  ## 9­13 (a)                   13­13
    3­3 (a)                             $ 9­13 (a)             13­13 (a)
                        5­5             $$ 9­13 (a)            13­13 (a)
    3­3 (a)      < 5­5                  ?? 9­13 (a)           13­13 (a)
                                                              / 13­13
   = 5­5                5­5                    9­13 (a)      // 13­13
                 = 5­5                         9­13 (a)              13­13 (a)
        5­5                                    9­13 (a)       · 13­13 (a)
                        5­5                    9­14           ÷ 13­13
   ::= 5­5        5­5                    10­10 (a)             13­13 (a)
                  5­5                   + 10­10 (a)                  13­13 (a)
   : = 5­5                             ++ 10­10 (a)           ^ 14­14
                        5­5             % 10­11              ^^ 14­14
   < 5­5                5­5            %% 10­11 (a)          .(2) 17­17 (a)
                  5­5                    | 10­11 (a)
   = 5­5          5­5                          10­11 (a)
                        5­5
   =| 5­5        |= 5­5
                ·(1) 5­14 (a)
   > 5­5        @@ 6­6 (a)
                : > 7­7
    5­5

        5­5

   ==.  5­5
        5­5

    5­5

        5­5

    5­5

   / 5­5

        ^+ 15­15    Postfix Operators                        15­15
                  ^* 15­15 ^# 15­15

   (1) Action composition (\cdot).
   (2) Record field (period).
   Operators Defined in Standard Modules.

   Modules Naturals, Integers, Reals

   +   - (1)               / (2)  ^ (3)                        ..    Nat  Real (2)

   ÷%                             <                            >     Int (4) Infinity (2)

                     (1) Only infix - is defined in Naturals.
                     (2) Defined only in Reals module.
                     (3) Exponentiation.
                     (4) Not defined in Naturals module.



-  Module Sequences

               Head SelectSeq SubSeq

   Append Len              Seq                                 Tail

   Module FiniteSets
         IsFiniteSet Cardinality

   Module Bags             BagIn                               CopiesIn   SubBag
                           BagOfAll                            EmptyBag
                           BagToSet                            IsABag
         BagCardinality    BagUnion                            SetToBag

   Module RealTime                now (declared to be a variable)
         RTBound RTnow

   Module TLC

   :>       @@             Print  Assert                       JavaTime   Permutations

   SortSeq
   ASCII Representation of Typeset Symbols

    /\ or \land                \/ or \lor              =>

   ¬ ~ or \lnot or \neg  <=> or \equiv = ==

    \in                       / \notin                = # or /=

   <<                         >>                      2 []

   <<                         >>                      3 <>

    \leq or =< or <=           \geq or >=             ; ~>

   \ll                        \gg                     -+ -+->
                              \succ                    |->
    \prec

-  \preceq                    \succeq                 ÷ \div

    \subseteq                  \supseteq              · \cdot

    \subset                    \supset                 \o or \circ
   < \sqsubset                = \sqsupset             · \bullet

   \sqsubseteq                \sqsupseteq                  \star

   |-                         -|                           \bigcirc

   |= |=                      =| =|                    \sim

    ->                         <-                          \simeq

    \cap or \intersect  \cup or \union                     \asymp

          \sqcap                    \sqcup             \approx
    (+) or \oplus                   \uplus
                              × \X or \times          ==.  \cong
          (-) or \ominus                                   \doteq

   (.) or \odot               \wr                     x y x^y (2)

    (\X) or \otimes            \propto                x + x^+ (2)
          (/) or \oslash      "s" "s" (1)             x  x^* (2)

    \E                         \A                     x# x^# (2)
    \EE                        \AA                        '

   ]v ]_v                     v >>_v

   WFv WF_v                   SFv SF_v

                -------- (3)                          -------- (3)

                -------- (3)                          ======== (3)

   (1) s is a sequence of characters.
   (2) x and y are any expressions.
   (3) a sequence of four or more - or = characters.
