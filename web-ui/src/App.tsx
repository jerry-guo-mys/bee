import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import Layout from '@/components/Layout';
import Dashboard from '@/pages/Dashboard';
import Agents from '@/pages/Agents';
import Workflows from '@/pages/Workflows';
import Monitoring from '@/pages/Monitoring';
import Settings from '@/pages/Settings';
import ToolPolicies from '@/pages/ToolPolicies';

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
        path: 'workflows',
        element: <Workflows />,
      },
      {
        path: 'monitoring',
        element: <Monitoring />,
      },
      {
        path: 'tool-policies',
        element: <ToolPolicies />,
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
