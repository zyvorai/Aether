import { useState, useEffect, useCallback } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import BarChart from '../BarChart';
import EmptyState from '../EmptyState';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import type { AffinityScore } from '../../types/api';

const WORKLOAD_CLASSES = [
  'web-service',
  'database',
  'ml-training',
  'microservice',
  'api-backend',
  'cache',
  'batch-job',
  'worker',
];

export default function AffinityPage() {
  const [affinityData, setAffinityData] = useState<Record<string, AffinityScore[]>>({});
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const load = useCallback(async () => {
    const results: Record<string, AffinityScore[]> = {};
    await Promise.all(
      WORKLOAD_CLASSES.map(async (cls) => {
        const data = await apiFetch<AffinityScore[]>(`/affinity/${cls}`);
        if (data) results[cls] = data;
      }),
    );
    setAffinityData(results);
  }, []);

  useEffect(() => {
    void (async () => {
      await load();
      setLoading(false);
    })();
  }, [load]);

  async function handleRefresh() {
    setRefreshing(true);
    await load();
    setRefreshing(false);
  }

  if (loading) {
    return <PageLoading rows={6} />;
  }

  const classes = Object.entries(affinityData);

  return (
    <div>
      <PageToolbar onRefresh={() => void handleRefresh()} refreshing={refreshing} />

      {classes.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No affinity data" description="Affinity scores are not available" />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {classes.map(([cls, scores]) => (
            <div key={cls} className="dash-card">
              <h2 className="text-lg font-semibold text-zinc-100 mb-4 capitalize">
                {cls.replace(/-/g, ' ')}
              </h2>
              <div className="space-y-3">
                {scores.map((s) => (
                  <div key={s.runtime} className="space-y-1">
                    <BarChart
                      label={s.runtime}
                      percent={s.composite_score * 100}
                      detail={`${s.total_deployments} deploys`}
                    />
                    <div className="flex gap-3 text-xs text-zinc-500 pl-1">
                      <span>Confidence: {(s.confidence * 100).toFixed(0)}%</span>
                      <span>Success: {(s.success_rate * 100).toFixed(0)}%</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
