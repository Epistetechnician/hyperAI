import type { SerendipityRecord } from "@justai/decision-contracts";

export type { SerendipityRecord };
export interface SerendipityBudget {
  declared_before_assessment: boolean;
  candidate_limit: number;
  preserved_alternative_limit: number;
}
