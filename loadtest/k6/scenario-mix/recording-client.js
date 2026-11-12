// The load CLI substitutes this module with k6/x/sqlite-recorder in its generated script.
// Plain k6 remains available for existing unrecorded experiments and inspect.
if (__ENV.SQLITE_RECORDER_REQUIRED) throw new Error("Use the load command with the custom k6-sqlite binary");
export default { record() {}, stats() { return {}; } };
