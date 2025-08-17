import { htmlReport } from 'https://raw.githubusercontent.com/benc-uk/k6-reporter/main/dist/bundle.js';
import http from "k6/http";
import { check, sleep } from "k6";
import { SharedArray } from "k6/data";

/**
 * DATA LOADING
 * ------------
 * We load dataset.json once per k6 instance (SharedArray). Each VU will
 * randomly pick an entry every iteration. Some entries contain an "inputs"
 * array directly; others contain "inputs_template" (base + repeat) which
 * we expand to generate long texts at runtime (so the dataset file stays small).
 */
const RAW_DATA = new SharedArray("dataset", () => JSON.parse(open("./dataset.json")));

/**
 * TEST CONFIG
 * -----------
 * Use constant-arrival-rate (open model) so we don't under-measure latency
 * during overload (avoids coordinated omission). Control RPS, duration, etc.
 *
 * Env vars (override defaults):
 *   URL  - target endpoint, e.g. http://localhost:8080/embed
 *   RPS  - requests per second, e.g. 300
 *   DUR  - duration, e.g. 2m
 *   VUS  - initial preAllocated VUs, e.g. 100
 *   MAX_VUS - max VUs k6 may scale to, e.g. 2000
 */
export const options = {
  scenarios: {
    open_model: {
      executor: "constant-arrival-rate",
      rate: Number(__ENV.RPS || 10),
      timeUnit: "1s",
      duration: __ENV.DUR || "1m",
      preAllocatedVUs: Number(__ENV.VUS || 10),
      maxVUs: Number(__ENV.MAX_VUS || 30),
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.01"],          // <1% failures
  },
};

const URL = __ENV.URL || `http://${__ENV.BIND_HOST}/embed`;
const HEADERS = { "Content-Type": "application/json" };

/**
 * buildPayload(entry)
 * -------------------
 * Converts a dataset entry into the POST body expected by TEI:
 *   - If "inputs" is present, uses it directly.
 *   - If "inputs_template" is present, generates a long string by repeating
 *     the provided base N times, then wraps it into "inputs": [generated].
 *   - Copies optional flags: normalize, truncate, truncation_direction.
 *
 * We intentionally omit "prompt_name" per your request.
 */
function buildPayload(entry) {
  let input;

  // Case 1: real inputs are provided
  if (entry.input) {
    input = entry.input;
  }
  // Case 2: generate long input from template (to trigger truncation paths)
  else if (entry.input_template && entry.input_template.base && entry.input_template.repeat) {
    const base = String(entry.input_template.base);
    const repeat = Number(entry.input_template.repeat);
    // Build a long string by repeating the base fragment
    let longText = base.repeat(repeat);
    input = longText;
  } else {
    // Fallback to a safe default; shouldn't happen with the provided dataset
    input = "fallback text";
  }

  // Assemble the body, including optional fields if present
  const body = { input };

  if (typeof entry.normalize === "boolean") {
    body.normalize = entry.normalize; // default is true on server; we set explicitly to vary behavior
  }
  if (typeof entry.truncate === "boolean") {
    body.truncate = entry.truncate;   // when true, over-long inputs are cut to model max length
  }
  if (typeof entry.truncation_direction === "string") {
    // TEI commonly accepts "left" or "right"; default is "right"
    body.truncation_direction = entry.truncation_direction;
  }

  return body;
}

/**
 * default()
 * ---------
 * Each iteration:
 *  - pick a random dataset entry
 *  - build the request payload
 *  - POST to the target URL
 *  - basic checks to record success/failure
 *
 * NOTE: We don't parse response JSON here to minimize client overhead
 * and keep the focus on service latency/throughput. If you want to assert
 * on embedding shape, you can JSON.parse(res.body) and check lengths.
 */
export default function () {
  const entry = RAW_DATA[Math.floor(Math.random() * RAW_DATA.length)];
  const payload = buildPayload(entry);

  const res = http.post(URL, JSON.stringify(payload), { headers: HEADERS });

  check(res, {
    "status is 200": (r) => r.status === 200,
  });

  // Tiny sleep helps avoid a tight CPU loop on very low RPS runs
  // (no effect under constant-arrival-rate with higher RPS)
  sleep(0.001);
}

export function handleSummary(data) {
  return {
    "summary.html": htmlReport(data),
  };
}
