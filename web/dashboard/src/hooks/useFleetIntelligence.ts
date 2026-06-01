// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
import { apiFetch } from '../utils/api';
import type { CommandCenterBriefingData } from '../components/CommandCenterBriefing';
import type { CostOptimizeReport, PredictionReport, ThreatReport, WorkloadResponse } from '../types/api';

export interface FleetIntelligenceSnapshot {
  loading: boolean;
  workloads: WorkloadResponse[];
  briefing: CommandCenterBriefingData | null;
  predictions: PredictionReport | null;
  cost: CostOptimizeReport | null;
  threats: ThreatReport | null;
  healthy: number;
  riskCount: number;
  savings: number;
  securityIssues: number;
  clusterCount: number;
}

export function useFleetIntelligence(refreshKey = 0): FleetIntelligenceSnapshot {
  const [loading, setLoading] = useState(true);
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [briefing, setBriefing] = useState<CommandCenterBriefingData | null>(null);
  const [predictions, setPredictions] = useState<PredictionReport | null>(null);
  const [cost, setCost] = useState<CostOptimizeReport | null>(null);
  const [threats, setThreats] = useState<ThreatReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    Promise.all([
      apiFetch<WorkloadResponse[]>('/workloads'),
      apiFetch<CommandCenterBriefingData>('/command-center/briefing'),
      apiFetch<PredictionReport>('/intelligence/predictions'),
      apiFetch<CostOptimizeReport>('/intelligence/cost-optimize'),
      apiFetch<ThreatReport>('/intelligence/threats'),
    ]).then(([w, b, p, c, t]) => {
      if (cancelled) return;
      setWorkloads(w ?? []);
      setBriefing(b);
      setPredictions(p);
      setCost(c);
      setThreats(t);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  const healthy = workloads.filter((w) => w.status.toLowerCase() === 'running').length;
  const riskCount =
    predictions?.predictions.filter((p) => p.risk_level === 'high' || p.risk_level === 'critical').length ?? 0;
  const savings = cost?.recommendations.reduce((sum, r) => sum + r.savings_monthly_usd, 0) ?? 0;
  const securityIssues = threats?.threats.length ?? 0;
  const clusterCount = new Set(workloads.map((w) => w.cluster).filter(Boolean)).size;

  return {
    loading,
    workloads,
    briefing,
    predictions,
    cost,
    threats,
    healthy,
    riskCount,
    savings,
    securityIssues,
    clusterCount,
  };
}
