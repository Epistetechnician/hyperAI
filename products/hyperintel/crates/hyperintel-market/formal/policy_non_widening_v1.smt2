; HyperIntel local abstract policy-non-widening check.
; Two candidates stand for the canonical report order used by routing.rs.
; Rank abstracts the complete deterministic routing tuple into a nonnegative total order.
(set-logic QF_LIA)

(declare-const eligible_0 Bool)
(declare-const eligible_1 Bool)
(declare-const has_cost_0 Bool)
(declare-const has_cost_1 Bool)
(declare-const rank_0 Int)
(declare-const rank_1 Int)

(assert (>= rank_0 0))
(assert (>= rank_1 0))

; The implemented abstraction filters on eligibility and available cost before minimization.
(define-fun admissible_0 () Bool (and eligible_0 has_cost_0))
(define-fun admissible_1 () Bool (and eligible_1 has_cost_1))
(define-fun selected_0 () Bool
  (and admissible_0 (or (not admissible_1) (<= rank_0 rank_1))))
(define-fun selected_1 () Bool
  (and admissible_1 (or (not admissible_0) (< rank_1 rank_0))))

; No selected candidate can be ineligible.
(push)
(assert
  (or
    (and selected_0 (not eligible_0))
    (and selected_1 (not eligible_1))))
(check-sat)
(pop)

; Counterexample replay: a weakened selector ignores eligibility and picks the cheaper cost-bearing
; candidate. The fixed assignment makes candidate 0 ineligible, cheaper, and selected.
(define-fun weak_selected_0 () Bool
  (and has_cost_0 (or (not has_cost_1) (<= rank_0 rank_1))))
(define-fun weak_selected_1 () Bool
  (and has_cost_1 (or (not has_cost_0) (< rank_1 rank_0))))
(push)
(assert (= eligible_0 false))
(assert (= eligible_1 true))
(assert (= has_cost_0 true))
(assert (= has_cost_1 true))
(assert (= rank_0 0))
(assert (= rank_1 1))
(assert
  (or
    (and weak_selected_0 (not eligible_0))
    (and weak_selected_1 (not eligible_1))))
(check-sat)
(pop)
