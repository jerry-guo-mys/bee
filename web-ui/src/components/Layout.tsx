import { useState, type ComponentType } from 'react';
import { Outlet, Link, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import {
  LayoutDashboard,
  Bot,
  Kanban,
  BarChart3,
  Settings,
  Shield,
  ChevronLeft,
  ChevronRight,
  Bell,
  Search,
  Menu,
  X,
  Building2,
  Building,
  Users,
  UserCheck,
  FileText,
  GitBranch,
  FolderKanban,
} from 'lucide-react';
import { cn } from '@/lib/utils';

type NavItem = { name: string; href: string; icon: ComponentType<{ className?: string }> };

const workbenchNav: NavItem[] = [
  { name: '总览', href: '/', icon: LayoutDashboard },
  { name: '流程与任务', href: '/workbench/runs', icon: Kanban },
  { name: '项目', href: '/workbench/projects', icon: FolderKanban },
];

const adminNav: NavItem[] = [
  { name: '流程模板', href: '/admin/workflow-templates', icon: GitBranch },
  { name: 'Agent 管理', href: '/agents', icon: Bot },
  { name: '工具策略', href: '/tool-policies', icon: Shield },
  { name: '监控日志', href: '/monitoring', icon: BarChart3 },
  { name: '租户管理', href: '/tenants', icon: Building2 },
  { name: '组织管理', href: '/organizations', icon: Building },
  { name: '团队管理', href: '/teams', icon: Users },
  { name: '成员管理', href: '/members', icon: UserCheck },
  { name: '审计日志', href: '/audit-logs', icon: FileText },
  { name: '系统设置', href: '/settings', icon: Settings },
];

function navItemActive(pathname: string, href: string): boolean {
  if (href === '/') return pathname === '/';
  return pathname === href || pathname.startsWith(`${href}/`);
}

function NavGroup({
  title,
  items,
  onNavigate,
}: {
  title: string;
  items: NavItem[];
  onNavigate: () => void;
}) {
  const location = useLocation();
  return (
    <div className="mt-4 first:mt-0">
      <p className="px-3 mb-2 text-[11px] font-semibold uppercase tracking-wide text-surface-400 dark:text-surface-500">
        {title}
      </p>
      <div className="space-y-1">
        {items.map((item) => {
          const isActive = navItemActive(location.pathname, item.href);
          return (
            <Link
              key={item.name}
              to={item.href}
              onClick={onNavigate}
              className={cn(
                'flex items-center gap-3 px-3 py-2.5 rounded-lg',
                'transition-all duration-200',
                'text-sm font-medium',
                isActive
                  ? 'bg-primary-50 text-primary-700 dark:bg-primary-900/20 dark:text-primary-400'
                  : 'text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-700'
              )}
            >
              <item.icon className={cn('w-5 h-5', isActive ? 'text-primary-600' : '')} />
              {item.name}
            </Link>
          );
        })}
      </div>
    </div>
  );
}

export default function Layout() {
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  return (
    <div className="min-h-screen bg-surface-50 dark:bg-surface-900">
      {/* Mobile menu button */}
      <button
        className="lg:hidden fixed top-4 left-4 z-50 p-2 rounded-lg bg-white dark:bg-surface-800 shadow-lg"
        onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
      >
        {mobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
      </button>

      {/* Sidebar */}
      <AnimatePresence>
        {(sidebarOpen || mobileMenuOpen) && (
          <>
            {/* Backdrop for mobile */}
            {mobileMenuOpen && (
              <div
                className="fixed inset-0 bg-black/50 z-40 lg:hidden"
                onClick={() => setMobileMenuOpen(false)}
              />
            )}

            <motion.aside
              initial={{ x: -280 }}
              animate={{ x: 0 }}
              exit={{ x: -280 }}
              transition={{ type: 'spring', damping: 25, stiffness: 200 }}
              className={cn(
                'fixed top-0 left-0 z-40 h-screen',
                'bg-white dark:bg-surface-800',
                'border-r border-surface-200 dark:border-surface-700',
                'lg:w-64 w-72',
                mobileMenuOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'
              )}
            >
              {/* Logo */}
              <div className="h-16 flex items-center px-6 border-b border-surface-200 dark:border-surface-700">
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-lg bg-primary-600 flex items-center justify-center">
                    <span className="text-white font-bold text-sm">B</span>
                  </div>
                  <span className="font-semibold text-lg text-surface-900 dark:text-surface-100">
                    Bee 控制台
                  </span>
                </div>
              </div>

              {/* Navigation */}
              <nav className="p-4 pb-8">
                <NavGroup
                  title="工作台"
                  items={workbenchNav}
                  onNavigate={() => setMobileMenuOpen(false)}
                />
                <NavGroup
                  title="平台管理"
                  items={adminNav}
                  onNavigate={() => setMobileMenuOpen(false)}
                />
              </nav>

              {/* Collapse button (desktop only) */}
              <button
                className="hidden lg:flex absolute top-4 right-2 p-1.5 rounded-lg text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-700"
                onClick={() => setSidebarOpen(!sidebarOpen)}
              >
                {sidebarOpen ? <ChevronLeft className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
              </button>
            </motion.aside>
          </>
        )}
      </AnimatePresence>

      {/* Main content */}
      <div
        className={cn(
          'transition-all duration-300',
          sidebarOpen ? 'lg:ml-64' : 'lg:ml-0'
        )}
      >
        {/* Top header */}
        <header className="h-16 bg-white dark:bg-surface-800 border-b border-surface-200 dark:border-surface-700 sticky top-0">
          <div className="h-full px-6 flex items-center justify-between">
            {/* Search */}
            <div className="flex items-center gap-4 flex-1">
              <div className="relative max-w-md w-full">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-surface-400" />
                <input
                  type="text"
                  placeholder="搜索任务、Agent、日志…"
                  className="w-full pl-10 pr-4 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                />
              </div>
            </div>

            {/* Right actions */}
            <div className="flex items-center gap-4">
              <button className="relative p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700">
                <Bell className="w-5 h-5 text-surface-500" />
                <span className="absolute top-1.5 right-1.5 w-2 h-2 bg-error-500 rounded-full" />
              </button>
              <div className="w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
                <span className="text-sm font-medium text-primary-700 dark:text-primary-400">U</span>
              </div>
            </div>
          </div>
        </header>

        {/* Page content */}
        <main className="p-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
