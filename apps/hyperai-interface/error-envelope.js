/* Normalize the local service's versioned errors without displaying body text. */
(() => {
  const GENERIC_MESSAGE = "The local HyperAI contract operation could not complete.";
  const ERROR_CODE = /^[A-Z0-9_-]{1,80}$/;
  const CODE_MESSAGES = Object.freeze({
    MARKET_SUPPLY_COMPARISON_CHANGED: "This supply comparison is out of date. Reload this page to get a fresh comparison before requesting a quote.",
  });

  function normalize(status, body) {
    const validStatus = Number.isInteger(status) && status >= 100 && status <= 599;
    const validEnvelope = body !== null
      && typeof body === "object"
      && !Array.isArray(body)
      && body.schema_version === "hyperai.interface.error.v1"
      && body.status === "error";
    const validCode = validEnvelope && typeof body.code === "string" && ERROR_CODE.test(body.code);
    const fixedMessage = validCode && Object.prototype.hasOwnProperty.call(CODE_MESSAGES, body.code)
      ? CODE_MESSAGES[body.code]
      : GENERIC_MESSAGE;
    return Object.freeze({
      status: validStatus ? status : 0,
      code: validCode ? body.code : "UNKNOWN_ERROR",
      message: fixedMessage,
    });
  }

  Object.defineProperty(globalThis, "HyperAIErrorEnvelope", {
    configurable: false,
    enumerable: false,
    value: Object.freeze({ normalize }),
    writable: false,
  });
})();
