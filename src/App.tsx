import './App.css';

import { AppShell } from './components/layout/AppShell';
import { useAppController } from './hooks/app/useAppController';

export default function App() {
  const appShellProps = useAppController();
  return <AppShell {...appShellProps} />;
}
