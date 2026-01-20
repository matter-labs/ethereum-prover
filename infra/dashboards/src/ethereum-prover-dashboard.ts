import { writeFileSync } from "node:fs";
import path from "node:path";

import * as common from "@grafana/grafana-foundation-sdk/common";
import * as dashboard from "@grafana/grafana-foundation-sdk/dashboard";
import * as prometheus from "@grafana/grafana-foundation-sdk/prometheus";
import { PanelBuilder as StatBuilder } from "@grafana/grafana-foundation-sdk/stat";
import { PanelBuilder as TimeseriesBuilder } from "@grafana/grafana-foundation-sdk/timeseries";
import * as units from "@grafana/grafana-foundation-sdk/units";

const DATASOURCE_UID = "PROMETHEUS_DS";

const prometheusQuery = (query: string, legend: string): prometheus.DataqueryBuilder => {
  return new prometheus.DataqueryBuilder()
    .datasource({ uid: DATASOURCE_UID, type: "prometheus" })
    .expr(query)
    .legendFormat(legend);
};

const withInstanceSelector = (metric: string): string => `${metric}{instance=~"$instance"}`;

const defaultTimeseries = (): TimeseriesBuilder => {
  return new TimeseriesBuilder()
    .height(8)
    .span(12)
    .lineWidth(1)
    .fillOpacity(10)
    .pointSize(4)
    .showPoints(common.VisibilityMode.Auto)
    .drawStyle(common.GraphDrawStyle.Line)
    .gradientMode(common.GraphGradientMode.None)
    .spanNulls(false)
    .axisBorderShow(false)
    .legend(
      new common.VizLegendOptionsBuilder()
        .showLegend(true)
        .placement(common.LegendPlacement.Bottom)
        .displayMode(common.LegendDisplayMode.List)
    )
    .tooltip(
      new common.VizTooltipOptionsBuilder()
        .mode(common.TooltipDisplayMode.Multi)
        .sort(common.SortOrder.Descending)
    )
    .thresholdsStyle(
      new common.GraphThresholdsStyleConfigBuilder()
        .mode(common.GraphThresholdsStyleMode.Off)
    );
};

const defaultStat = (): StatBuilder => {
  return new StatBuilder()
    .height(8)
    .span(6)
    .decimals(0)
    .reduceOptions(
      new common.ReduceDataOptionsBuilder()
        .calcs(["lastNotNull"])
    )
    .colorMode(common.BigValueColorMode.Value)
    .graphMode(common.BigValueGraphMode.None);
};

const timeseriesPanel = (
  title: string,
  unit: string,
  targets: prometheus.DataqueryBuilder[],
  span = 12,
  height = 8
): TimeseriesBuilder => {
  let panel = defaultTimeseries().title(title).unit(unit).span(span).height(height);
  for (const target of targets) {
    panel = panel.withTarget(target);
  }
  return panel;
};

const statPanel = (
  title: string,
  unit: string | null,
  targets: prometheus.DataqueryBuilder[],
  span = 6,
  height = 8
): StatBuilder => {
  let panel = defaultStat().title(title).span(span).height(height);
  if (unit) {
    panel = panel.unit(unit);
  }
  for (const target of targets) {
    panel = panel.withTarget(target);
  }
  return panel;
};

const builder = new dashboard.DashboardBuilder("Ethereum Prover")
  .uid("ethereum-prover")
  .tags(["generated", "ethereum-prover"])
  .editable()
  .tooltip(dashboard.DashboardCursorSync.Off)
  .refresh("10s")
  .time({ from: "now-6h", to: "now" })
  .timezone("browser")
  .timepicker(
    new dashboard.TimePickerBuilder()
      .refreshIntervals(["5s", "10s", "30s", "1m", "5m", "15m", "30m", "1h", "2h", "1d"])
  )
  .withVariable(
    new dashboard.QueryVariableBuilder("instance")
      .label("Instance")
      .query("label_values(ethereum_prover_last_processed_block, instance)")
      .datasource({ uid: DATASOURCE_UID, type: "prometheus" })
      .current({
        selected: true,
        text: "All",
        value: "$__all",
      })
      .refresh(dashboard.VariableRefresh.OnTimeRangeChanged)
      .sort(dashboard.VariableSort.AlphabeticalAsc)
      .multi(true)
      .includeAll(true)
      .allValue(".*")
  )
  .withRow(new dashboard.RowBuilder("General metrics"))
  .withPanel(
    timeseriesPanel(
      "Last processed block (timeline)",
      "locale",
      [prometheusQuery(withInstanceSelector("ethereum_prover_last_processed_block"), "{{instance}}")]
    )
  )
  .withPanel(
    statPanel(
      "Last processed block",
      "locale",
      [prometheusQuery(withInstanceSelector("ethereum_prover_last_processed_block"), "{{instance}}")],
      12
    ).thresholds(
      new dashboard.ThresholdsConfigBuilder()
        .mode(dashboard.ThresholdsMode.Absolute)
        .steps([{ value: null, color: "green" }])
    )
  )
  .withRow(new dashboard.RowBuilder("Witnessing"))
  .withPanel(
    timeseriesPanel(
      "Witness duration (p50/p95/p99)",
      units.Seconds,
      [
        prometheusQuery(
          "histogram_quantile(0.5, sum(rate(ethereum_prover_witness_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p50 {{instance}}"
        ),
        prometheusQuery(
          "histogram_quantile(0.95, sum(rate(ethereum_prover_witness_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p95 {{instance}}"
        ),
        prometheusQuery(
          "histogram_quantile(0.99, sum(rate(ethereum_prover_witness_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p99 {{instance}}"
        ),
      ]
    )
  )
  .withPanel(
    timeseriesPanel(
      "Witness success/failure (5m count)",
      units.Short,
      [
        prometheusQuery(`increase(${withInstanceSelector("ethereum_prover_witness_success_total")}[5m])`, "success {{instance}}"),
        prometheusQuery(`increase(${withInstanceSelector("ethereum_prover_witness_failure_total")}[5m])`, "failure {{instance}}"),
      ],
      6
    )
  )
  .withPanel(
    statPanel(
      "Inflight witness tasks",
      units.Short,
      [prometheusQuery(withInstanceSelector("ethereum_prover_inflight_witness_tasks"), "{{instance}}")],
      6
    )
  )
  .withRow(new dashboard.RowBuilder("Proving"))
  .withPanel(
    timeseriesPanel(
      "Proof duration (p50/p95/p99)",
      units.Seconds,
      [
        prometheusQuery(
          "histogram_quantile(0.5, sum(rate(ethereum_prover_proof_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p50 {{instance}}"
        ),
        prometheusQuery(
          "histogram_quantile(0.95, sum(rate(ethereum_prover_proof_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p95 {{instance}}"
        ),
        prometheusQuery(
          "histogram_quantile(0.99, sum(rate(ethereum_prover_proof_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p99 {{instance}}"
        ),
      ]
    )
  )
  .withPanel(
    timeseriesPanel(
      "Proof success/failure (5m count)",
      units.Short,
      [
        prometheusQuery(`increase(${withInstanceSelector("ethereum_prover_proof_success_total")}[5m])`, "success {{instance}}"),
        prometheusQuery(`increase(${withInstanceSelector("ethereum_prover_proof_failure_total")}[5m])`, "failure {{instance}}"),
      ],
      6
    )
  )
  .withPanel(
    statPanel(
      "Inflight proof tasks",
      units.Short,
      [prometheusQuery(withInstanceSelector("ethereum_prover_inflight_proof_tasks"), "{{instance}}")],
      6
    )
  )
  .withRow(new dashboard.RowBuilder("EthProofs client"))
  .withPanel(
    timeseriesPanel(
      "EthProofs request duration (p50/p95/p99)",
      units.Seconds,
      [
        prometheusQuery(
          "histogram_quantile(0.5, sum(rate(ethereum_prover_ethproofs_request_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p50 {{instance}}"
        ),
        prometheusQuery(
          "histogram_quantile(0.95, sum(rate(ethereum_prover_ethproofs_request_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p95 {{instance}}"
        ),
        prometheusQuery(
          "histogram_quantile(0.99, sum(rate(ethereum_prover_ethproofs_request_duration_seconds_bucket{instance=~\"$instance\"}[5m])) by (le, instance))",
          "p99 {{instance}}"
        ),
      ]
    )
  )
  .withPanel(
    timeseriesPanel(
      "EthProofs success/failure (5m count)",
      units.Short,
      [
        prometheusQuery(`increase(${withInstanceSelector("ethereum_prover_ethproofs_request_success_total")}[5m])`, "success {{instance}}"),
        prometheusQuery(`increase(${withInstanceSelector("ethereum_prover_ethproofs_request_failure_total")}[5m])`, "failure {{instance}}"),
      ],
      6
    )
  )
  .withPanel(
    statPanel(
      "EthProofs failure ratio",
      units.PercentUnit,
      [
        prometheusQuery(
          "sum(rate(ethereum_prover_ethproofs_request_failure_total{instance=~\"$instance\"}[5m])) by (instance) / sum(rate(ethereum_prover_ethproofs_request_success_total{instance=~\"$instance\"}[5m]) + rate(ethereum_prover_ethproofs_request_failure_total{instance=~\"$instance\"}[5m])) by (instance)",
          "{{instance}}"
        ),
      ],
      6
    )
  );

const outputPath = path.resolve(__dirname, "../../grafana/dashboards/ethereum-prover.json");
writeFileSync(outputPath, JSON.stringify(builder.build(), null, 2));
