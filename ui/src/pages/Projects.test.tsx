// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { Project } from "@paperclipai/shared";
import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Projects } from "./Projects";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mockProjectsApi = vi.hoisted(() => ({
  list: vi.fn(),
}));
const mockOpenNewProject = vi.hoisted(() => vi.fn());
const mockSetBreadcrumbs = vi.hoisted(() => vi.fn());

vi.mock("../api/projects", () => ({ projectsApi: mockProjectsApi }));
vi.mock("@/lib/router", () => ({
  Link: ({ children, to }: { children?: ReactNode; to: string }) => <a href={to}>{children}</a>,
}));
vi.mock("../context/CompanyContext", () => ({
  useCompany: () => ({ selectedCompanyId: "company-1" }),
}));
vi.mock("../context/DialogContext", () => ({
  useDialogActions: () => ({ openNewProject: mockOpenNewProject }),
}));
vi.mock("../context/BreadcrumbContext", () => ({
  useBreadcrumbs: () => ({ setBreadcrumbs: mockSetBreadcrumbs }),
}));

function project(overrides: Partial<Project>): Project {
  const now = new Date("2026-05-01T00:00:00Z");
  return {
    id: "project-1",
    companyId: "company-1",
    urlKey: "project-1",
    goalId: null,
    goalIds: [],
    goals: [],
    name: "Project",
    description: null,
    status: "in_progress",
    leadAgentId: null,
    targetDate: null,
    color: "#14b8a6",
    env: null,
    pauseReason: null,
    pausedAt: null,
    executionWorkspacePolicy: null,
    codebase: {
      workspaceId: null,
      repoUrl: null,
      repoRef: null,
      defaultRef: null,
      repoName: null,
      localFolder: null,
      managedFolder: "/tmp/project-1",
      effectiveLocalFolder: "/tmp/project-1",
      origin: "managed_checkout",
    },
    workspaces: [],
    primaryWorkspace: null,
    archivedAt: null,
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

async function renderProjects(container: HTMLElement) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  let root: Root | null = null;

  await act(async () => {
    root = createRoot(container);
    root.render(
      <QueryClientProvider client={queryClient}>
        <Projects />
      </QueryClientProvider>,
    );
  });
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
    await new Promise((resolve) => setTimeout(resolve, 0));
  });

  return root;
}

function projectLinkNames(container: HTMLElement): string[] {
  return Array.from(container.querySelectorAll<HTMLAnchorElement>("a[href^='/projects/']")).map((link) => {
    const title = link.querySelector("span.truncate");
    return title?.textContent ?? "";
  });
}

describe("Projects", () => {
  let root: Root | null = null;
  let container: HTMLDivElement;

  beforeEach(() => {
    container = document.createElement("div");
    document.body.appendChild(container);
    mockProjectsApi.list.mockResolvedValue([
      project({
        id: "project-bravo",
        urlKey: "bravo",
        name: "Bravo",
        description: null,
        updatedAt: new Date("2026-05-02T00:00:00Z"),
      }),
      project({
        id: "project-alpha",
        urlKey: "alpha",
        name: "Alpha",
        description: "First project",
        updatedAt: new Date("2026-05-01T00:00:00Z"),
      }),
      project({
        id: "project-charlie",
        urlKey: "charlie",
        name: "Charlie",
        description: null,
        updatedAt: new Date("2026-05-03T00:00:00Z"),
      }),
    ]);
  });

  afterEach(() => {
    act(() => root?.unmount());
    root = null;
    container.remove();
    vi.clearAllMocks();
  });

  it("sorts projects by name by default and can switch sort mode", async () => {
    root = await renderProjects(container);

    expect(projectLinkNames(container)).toEqual(["Alpha", "Bravo", "Charlie"]);

    const select = container.querySelector("select");
    expect(select).not.toBeNull();

    await act(async () => {
      select!.value = "updated";
      select!.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(projectLinkNames(container)).toEqual(["Charlie", "Bravo", "Alpha"]);
  });

  it("reserves description line height for projects without descriptions", async () => {
    root = await renderProjects(container);

    const bravoLink = Array.from(container.querySelectorAll<HTMLAnchorElement>("a")).find((link) =>
      link.textContent?.includes("Bravo"),
    );
    const hiddenDescriptionLine = bravoLink?.querySelector("p[aria-hidden='true']");

    expect(hiddenDescriptionLine).not.toBeNull();
    expect(hiddenDescriptionLine?.className).toContain("min-h-4");
  });
});
