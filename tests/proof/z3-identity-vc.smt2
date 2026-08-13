(set-logic QF_LIA)
; Negation of identity's postcondition: result >= x with result = x.
(declare-const x Int)
(declare-const result Int)
(assert (= result x))
(assert (not (>= result x)))
(check-sat)
