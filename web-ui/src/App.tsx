import { createBrowserRouter, Navigate, RouterProvider } from 'react-router-dom';
import Layout from '@/components/Layout';
import Dashboard from '@/pages/Dashboard';
import Agents from '@/pages/Agents';
import Workflows from '@/pages/Workflows';
import Projects from '@/pages/Projects';
import Monitoring from '@/pages/Monitoring';
import Settings from '@/pages/Settings';
import ToolPolicies from '@/pages/ToolPolicies';
import Tenants from '@/pages/Tenants';
import Organizations from '@/pages/Organizations';
import Teams from '@/pages/Teams';
import Members from '@/pages/Members';
import AuditLogs from '@/pages/AuditLogs';
import WorkflowTemplatesSettings from '@/pages/WorkflowTemplatesSettings';

const router = createBrowserRouter([
  {
    path: '/',
    element: <Layout />,
    children: [
      {
        index: true,
        element: <Dashboard />,
      },
      {
        path: 'agents',
        element: <Agents />,
      },
      {
        path: 'workbench/runs',
        element: <Workflows />,
      },
      {
        path: 'workbench/projects',
        element: <Projects />,
      },
      {
        path: 'workbench',
        element: <Navigate to="/workbench/runs" replace />,
      },
      {
        path: 'workflows',
        element: <Navigate to="/workbench/runs" replace />,
      },
      {
        path: 'tool-policies',
        element: <ToolPolicies />,
      },
      {
        path: 'monitoring',
        element: <Monitoring />,
      },
      {
        path: 'tenants',
        element: <Tenants />,
      },
      {
        path: 'organizations',
        element: <Organizations />,
      },
      {
        path: 'teams',
        element: <Teams />,
      },
      {
        path: 'members',
        element: <Members />,
      },
      {
        path: 'audit-logs',
        element: <AuditLogs />,
      },
      {
        path: 'admin/workflow-templates',
        element: <WorkflowTemplatesSettings />,
      },
      {
        path: 'settings',
        element: <Settings />,
      },
    ],
  },
]);

export default function App() {
  return <RouterProvider router={router} />;
}
