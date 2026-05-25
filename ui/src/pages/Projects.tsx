import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import type { Project } from "@paperclipai/shared";
import { projectsApi } from "../api/projects";
import { useCompany } from "../context/CompanyContext";
import { useDialogActions } from "../context/DialogContext";
import { useBreadcrumbs } from "../context/BreadcrumbContext";
import { queryKeys } from "../lib/queryKeys";
import { EntityRow } from "../components/EntityRow";
import { StatusBadge } from "../components/StatusBadge";
import { EmptyState } from "../components/EmptyState";
import { PageSkeleton } from "../components/PageSkeleton";
import { formatDate, projectUrl } from "../lib/utils";
import { Button } from "@/components/ui/button";
import { ArrowUpDown, Hexagon, Plus } from "lucide-react";

type ProjectSortMode = "name" | "updated" | "targetDate" | "status";

const PROJECT_SORT_LABELS: Record<ProjectSortMode, string> = {
  name: "Name",
  updated: "Recently updated",
  targetDate: "Target date",
  status: "Status",
};

const PROJECT_STATUS_RANK: Record<Project["status"], number> = {
  in_progress: 0,
  planned: 1,
  backlog: 2,
  completed: 3,
  cancelled: 4,
};

function projectTime(project: Project, field: "createdAt" | "updatedAt"): number {
  const value = project[field];
  const time = value instanceof Date ? value.getTime() : new Date(value).getTime();
  return Number.isFinite(time) ? time : 0;
}

function compareProjectNames(left: Project, right: Project): number {
  const nameDiff = left.name.localeCompare(right.name, undefined, { sensitivity: "base" });
  return nameDiff !== 0 ? nameDiff : left.id.localeCompare(right.id);
}

function compareTargetDates(left: Project, right: Project): number {
  if (!left.targetDate && !right.targetDate) return compareProjectNames(left, right);
  if (!left.targetDate) return 1;
  if (!right.targetDate) return -1;

  const dateDiff = left.targetDate.localeCompare(right.targetDate);
  return dateDiff !== 0 ? dateDiff : compareProjectNames(left, right);
}

function sortProjects(projects: Project[], sortMode: ProjectSortMode): Project[] {
  return [...projects].sort((left, right) => {
    if (sortMode === "updated") {
      const updatedDiff = projectTime(right, "updatedAt") - projectTime(left, "updatedAt");
      return updatedDiff !== 0 ? updatedDiff : compareProjectNames(left, right);
    }
    if (sortMode === "targetDate") {
      return compareTargetDates(left, right);
    }
    if (sortMode === "status") {
      const statusDiff = PROJECT_STATUS_RANK[left.status] - PROJECT_STATUS_RANK[right.status];
      return statusDiff !== 0 ? statusDiff : compareProjectNames(left, right);
    }
    return compareProjectNames(left, right);
  });
}

export function Projects() {
  const { selectedCompanyId } = useCompany();
  const { openNewProject } = useDialogActions();
  const { setBreadcrumbs } = useBreadcrumbs();
  const [sortMode, setSortMode] = useState<ProjectSortMode>("name");

  useEffect(() => {
    setBreadcrumbs([{ label: "Projects" }]);
  }, [setBreadcrumbs]);

  const { data: allProjects, isLoading, error } = useQuery({
    queryKey: queryKeys.projects.list(selectedCompanyId!),
    queryFn: () => projectsApi.list(selectedCompanyId!),
    enabled: !!selectedCompanyId,
  });
  const projects = useMemo(
    () => (allProjects ?? []).filter((p) => !p.archivedAt),
    [allProjects],
  );
  const sortedProjects = useMemo(
    () => sortProjects(projects, sortMode),
    [projects, sortMode],
  );

  if (!selectedCompanyId) {
    return <EmptyState icon={Hexagon} message="Select a company to view projects." />;
  }

  if (isLoading) {
    return <PageSkeleton variant="list" />;
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <label className="flex items-center gap-2 text-sm text-muted-foreground">
          <ArrowUpDown className="h-4 w-4" />
          <span>Sort</span>
          <select
            className="rounded-md border border-border bg-background px-2.5 py-1.5 text-sm text-foreground outline-none transition-colors hover:bg-accent/50 focus-visible:ring-2 focus-visible:ring-ring"
            value={sortMode}
            onChange={(event) => setSortMode(event.target.value as ProjectSortMode)}
          >
            {(Object.keys(PROJECT_SORT_LABELS) as ProjectSortMode[]).map((value) => (
              <option key={value} value={value}>
                {PROJECT_SORT_LABELS[value]}
              </option>
            ))}
          </select>
        </label>
        <Button size="sm" variant="outline" onClick={openNewProject}>
          <Plus className="h-4 w-4 mr-1" />
          Add Project
        </Button>
      </div>

      {error && <p className="text-sm text-destructive">{error.message}</p>}

      {!isLoading && projects.length === 0 && (
        <EmptyState
          icon={Hexagon}
          message="No projects yet."
          action="Add Project"
          onAction={openNewProject}
        />
      )}

      {projects.length > 0 && (
        <div className="border border-border">
          {sortedProjects.map((project) => (
            <EntityRow
              key={project.id}
              title={project.name}
              subtitle={project.description ?? undefined}
              reserveSubtitleSpace
              to={projectUrl(project)}
              trailing={
                <div className="flex items-center gap-3">
                  {project.targetDate && (
                    <span className="text-xs text-muted-foreground">
                      {formatDate(project.targetDate)}
                    </span>
                  )}
                  <StatusBadge status={project.status} />
                </div>
              }
            />
          ))}
        </div>
      )}
    </div>
  );
}
