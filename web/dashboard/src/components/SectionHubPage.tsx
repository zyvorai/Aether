// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';
import { ArrowRight } from 'lucide-react';
import type { AppView } from '../types/api';
import { viewToPath } from '../utils/dashboardRoutes';
import { Link } from 'react-router';
import { Card, CardBody, CardHeader } from './ui/Card';

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
    <Card className="mb-6">
      <CardHeader>
        <p className="text-xs font-normal uppercase tracking-wide text-muted">Tools</p>
        <h2 className="mt-1 text-lg font-semibold text-foreground">{title}</h2>
        <p className="mt-1 max-w-2xl text-sm text-muted">{subtitle}</p>
      </CardHeader>
      <CardBody>
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {links.map((link) => (
            <Link
              key={link.view}
              to={viewToPath(link.view)}
              className="group glass glass-hover-lift block rounded-[var(--radius-liquid)] p-4 no-underline"
            >
              <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-xl border border-primary/20 bg-primary-wash text-primary">
                {link.icon}
              </div>
              <div className="flex items-start justify-between gap-2">
                <div>
                  <h3 className="font-medium text-foreground">{link.title}</h3>
                  <p className="mt-1 text-sm text-muted">{link.description}</p>
                </div>
                <ArrowRight className="mt-1 h-4 w-4 shrink-0 text-muted transition group-hover:text-primary" />
              </div>
            </Link>
          ))}
        </div>
      </CardBody>
    </Card>
  );
}
