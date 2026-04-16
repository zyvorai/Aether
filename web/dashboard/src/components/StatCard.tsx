import type { ReactNode } from 'react';

interface StatCardProps {
  title: string;
  value: string | number;
  color: 'orange' | 'green' | 'red' | 'blue' | 'purple' | 'yellow';
  icon?: ReactNode;
}

const colorMap: Record<StatCardProps['color'], string> = {
  orange: 'stat-card-orange',
  green: 'stat-card-green',
  red: 'stat-card-red',
  blue: 'stat-card-blue',
  purple: 'stat-card-purple',
  yellow: 'stat-card-yellow',
};

const borderMap: Record<StatCardProps['color'], string> = {
  orange: 'border-orange-500/20',
  green: 'border-emerald-500/20',
  red: 'border-red-500/20',
  blue: 'border-blue-500/20',
  purple: 'border-purple-500/20',
  yellow: 'border-amber-500/20',
};

export default function StatCard({ title, value, color, icon }: StatCardProps) {
  return (
    <div
      className={`${colorMap[color]} ${borderMap[color]} border rounded-xl p-5 card-glow transition-all duration-200 hover:scale-[1.02] relative overflow-hidden`}
    >
      {icon && (
        <div className="absolute top-4 right-4 text-zinc-600">
          {icon}
        </div>
      )}
      <p className="text-[11px] font-semibold text-zinc-400 uppercase tracking-wider mb-2">
        {title}
      </p>
      <p className="text-2xl font-bold text-white">{value}</p>
    </div>
  );
}
