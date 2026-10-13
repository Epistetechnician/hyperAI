import { execFileSync } from "node:child_process";

const PROCESS_STATE_PATTERN = /^[A-Za-z?][A-Za-z<>+]*$/;

/**
 * Parses the fixed `ps -axo pid=,pgid=,stat=` columns for one process group.
 * Rows from other groups and malformed rows are ignored.
 *
 * @param {string} output process table text with PID, PGID, and state columns
 * @param {number} processGroupId positive process-group identifier
 * @returns {{pid: number, processGroupId: number, state: string}[]}
 */
export function parseProcessGroupMembers(output, processGroupId) {
  if (!Number.isSafeInteger(processGroupId) || processGroupId < 1) return [];

  return output.split(/\r?\n/).flatMap((line) => {
    const fields = line.trim().split(/\s+/);
    if (fields.length !== 3
      || !/^\d+$/.test(fields[0])
      || !/^\d+$/.test(fields[1])
      || !PROCESS_STATE_PATTERN.test(fields[2])) {
      return [];
    }
    const pid = Number(fields[0]);
    const memberGroupId = Number(fields[1]);
    if (!Number.isSafeInteger(pid) || pid < 1
      || !Number.isSafeInteger(memberGroupId)
      || memberGroupId !== processGroupId) return [];
    return [{ pid, processGroupId: memberGroupId, state: fields[2].toUpperCase() }];
  });
}

/** Reads only PID, process-group ID, and process state from the OS process table. */
export function readProcessGroupMembers(processGroupId) {
  const output = execFileSync("ps", ["-axo", "pid=,pgid=,stat="], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  return parseProcessGroupMembers(output, processGroupId);
}

/** Defunct processes have exited and cannot execute or retain open files. */
export function hasLiveProcessGroupMembers(members) {
  return members.some(({ state }) => !state.startsWith("Z"));
}
