// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';
import { ArrowRight } from 'lucide-react';
import type { AppView } from '../types/api';
import { viewToPath } from '../utils/dashboardRoutes';
import { Link } from 'react-router';

export interface HubLink {
  view: AppView;
  title: string;
  description: string;
  icon: ReactNode;
}

interface SectionHubPageProps {
  title: string;
  subtitle: string;
  links: HubLink[];
}

export default function SectionHubPage({ title, subtitle, links }: SectionHubPageProps) {
  return (
    <section className="overview-section-shell p-6 sm:p-8">
      <div className="overview-section-header">
        <p className="section-label">Tools</p>
        <h2 className="section-title">{title}</h2>
        <p className="section-subtitle max-w-2xl">{subtitle}</p>
      </div>
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {links.map((link) => (
          <Link
            key={link.view}
            to={viewToPath(link.view)}
            className="group hub-link-card"
          >
            <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-xl border border-aether/20 bg-aether/10 text-aether">
              {link.icon}
            </div>
            <div className="flex items-start justify-between gap-2">
              <div>
                <h3 className="font-medium text-slate-100 group-hover:text-white">{link.title}</h3>
                <p className="mt-1 text-sm text-slate-500">{link.description}</p>
              </div>
              <ArrowRight className="mt-1 h-4 w-4 shrink-0 text-slate-600 transition group-hover:text-aether" />
            </div>
          </Link>
        ))}
      </div>
    </section>
  );
}
