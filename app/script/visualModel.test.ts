import test from "node:test";
import assert from "node:assert/strict";

import {
  buildMetricRangeModel,
  buildSourceQualitySegments,
  buildStatusGlyphModel,
  evidenceMeterFromLine,
} from "../src/visualModel.ts";

test("metric range model labels every marked value with units", () => {
  const model = buildMetricRangeModel({
    channel: "Press.2s",
    unit: "cmH2O",
    samples: 500,
    min: 10,
    mean: 14,
    median: 15,
    p95: 18,
    max: 20,
  });

  assert.equal(model.axisLabel, "Press.2s · cmH2O");
  assert.equal(model.strokeWidth, 2.8);
  assert.deepEqual(
    model.markers.map((marker) => marker.label),
    ["min 10.0 cmH2O", "mean 14.0 cmH2O", "median 15.0 cmH2O", "p95 18.0 cmH2O", "max 20.0 cmH2O"],
  );
  assert.deepEqual(
    model.markers.map((marker) => marker.x),
    [0, 40, 50, 80, 100],
  );
});

test("metric range model handles flat signals without collapsing labels", () => {
  const model = buildMetricRangeModel({
    channel: "Leak.2s",
    unit: "L/sec",
    samples: 10,
    min: 0,
    mean: 0,
    median: 0,
    p95: 0,
    max: 0,
  });

  assert.equal(model.strokeWidth, 1.6);
  assert.equal(model.markers[0].x, 50);
  assert.equal(model.markers[model.markers.length - 1].x, 50);
  assert.ok(model.markers.every((marker) => marker.label.endsWith("L/sec")));
});

test("source quality segments preserve accepted, limited, parse error, and rejected proportions", () => {
  const segments = buildSourceQualitySegments({
    totalFiles: 20,
    acceptedFiles: 16,
    rejectedFiles: 4,
    validEdfFiles: 10,
    limitedEdfFiles: 3,
    parseErrorEdfFiles: 1,
  });

  assert.deepEqual(
    segments.map((segment) => [segment.kind, segment.value, segment.percent]),
    [
      ["valid", 10, 50],
      ["limited", 3, 15],
      ["parse_error", 1, 5],
      ["accepted_other", 2, 10],
      ["rejected", 4, 20],
    ],
  );
});

test("status glyph model maps analysis confidence to visual line grammar", () => {
  assert.deepEqual(buildStatusGlyphModel("available"), {
    lineStyle: "solid",
    strokeWidth: 2.6,
    marker: "filled",
  });
  assert.deepEqual(buildStatusGlyphModel("limited"), {
    lineStyle: "dotted",
    strokeWidth: 1.8,
    marker: "hollow",
  });
  assert.deepEqual(buildStatusGlyphModel("gated"), {
    lineStyle: "dashed",
    strokeWidth: 1.2,
    marker: "barred",
  });
});

test("evidence meter reads leading sample counts without grabbing channel suffixes", () => {
  const meter = evidenceMeterFromLine(
    "Compared 120 aligned samples from decoded Leak.2s and Press.2s physical values.",
  );

  assert.deepEqual(meter, {
    label: "Compared 120 aligned samples",
    valueLabel: "120",
    percent: 69.4,
  });
});
