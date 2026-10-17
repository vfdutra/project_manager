import { test } from "node:test";
import assert from "node:assert/strict";
import { reduce, projectRunning, projectUrl, portOf, cmdStatus, projectStatus, commandUrl, MAX_LOG, type Runtime } from "../src/runtime.ts";

const P = "C:\\Projetos\\Node\\web";
const project = { name: "web", path: P, kind: "node", commands: [], url: null, customized: false };

test("started → log → url → exited", () => {
  let rt: Runtime = {};
  rt = reduce(rt, { type: "started", project: P, command: "dev" });
  rt = reduce(rt, { type: "log", project: P, command: "dev", line: "hello" });
  rt = reduce(rt, { type: "url", project: P, command: "dev", url: "http://localhost:5173" });
  assert.equal(projectRunning(rt, P), true);
  assert.deepEqual(rt[P].dev.logs, ["hello"]);
  assert.equal(projectUrl(rt, project), "http://localhost:5173");

  rt = reduce(rt, { type: "exited", project: P, command: "dev", code: 1 });
  assert.equal(projectRunning(rt, P), false);
  assert.equal(rt[P].dev.code, 1);
  assert.equal(projectUrl(rt, project), null);
});

test("new run clears previous logs", () => {
  let rt: Runtime = {};
  rt = reduce(rt, { type: "started", project: P, command: "dev" });
  rt = reduce(rt, { type: "log", project: P, command: "dev", line: "old" });
  rt = reduce(rt, { type: "started", project: P, command: "dev" });
  assert.deepEqual(rt[P].dev.logs, []);
});

test("override url wins over detected url", () => {
  let rt: Runtime = reduce({}, { type: "started", project: P, command: "dev" });
  rt = reduce(rt, { type: "url", project: P, command: "dev", url: "http://localhost:5173" });
  assert.equal(projectUrl(rt, { ...project, url: "http://localhost:8080" }), "http://localhost:8080");
});

test("logs are capped", () => {
  let rt: Runtime = reduce({}, { type: "started", project: P, command: "dev" });
  for (let i = 0; i < MAX_LOG + 100; i++) rt = reduce(rt, { type: "log", project: P, command: "dev", line: `l${i}` });
  assert.equal(rt[P].dev.logs.length, MAX_LOG);
  assert.equal(rt[P].dev.logs.at(-1), `l${MAX_LOG + 99}`);
});

test("portOf", () => {
  assert.equal(portOf("http://localhost:5173"), "5173");
  assert.equal(portOf("https://localhost"), "443");
  assert.equal(portOf("nope"), "?");
});

test("status dots: running, crashed, stopped by user", () => {
  let rt: Runtime = reduce({}, { type: "started", project: P, command: "a" });
  rt = reduce(rt, { type: "started", project: P, command: "b" });
  assert.equal(projectStatus(rt, P), "on");
  rt = reduce(rt, { type: "exited", project: P, command: "a", code: 1 });
  assert.equal(cmdStatus(rt[P].a), "err");
  assert.equal(projectStatus(rt, P), "on");
  rt = reduce(rt, { type: "exited", project: P, command: "b", code: null });
  assert.equal(cmdStatus(rt[P].b), "");
  assert.equal(projectStatus(rt, P), "err");
  assert.equal(projectStatus({}, P), "");
});

test("commandUrl only while that command runs", () => {
  let rt: Runtime = reduce({}, { type: "started", project: P, command: "front" });
  rt = reduce(rt, { type: "url", project: P, command: "front", url: "http://localhost:5173" });
  assert.equal(commandUrl(rt, P, "front"), "http://localhost:5173");
  assert.equal(commandUrl(rt, P, "back"), null);
  rt = reduce(rt, { type: "exited", project: P, command: "front", code: null });
  assert.equal(commandUrl(rt, P, "front"), null);
});
