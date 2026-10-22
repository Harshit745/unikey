import React, { lazy, Suspense } from 'react';
import { HashRouter, Routes, Route } from 'react-router-dom';
import { DaemonProvider } from '@/context/DaemonContext';
import { AppearanceProvider } from '@/context/AppearanceContext';
import { Layout } from '@/components/Layout';
import { ErrorBoundary } from '@/components/ErrorBoundary';

const Dashboard = lazy(() => import('@/pages/Dashboard'));
const Devices   = lazy(() => import('@/pages/Devices'));
const Pairing   = lazy(() => import('@/pages/Pairing'));
const Clipboard = lazy(() => import('@/pages/Clipboard'));
const Input     = lazy(() => import('@/pages/Input'));
const Files     = lazy(() => import('@/pages/Files'));
const Settings  = lazy(() => import('@/pages/Settings'));
const Logs      = lazy(() => import('@/pages/Logs'));
const About     = lazy(() => import('@/pages/About'));

const Spinner = () => (
  <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
    <div className="spinner" />
  </div>
);

const App: React.FC = () => (
  <ErrorBoundary>
    <DaemonProvider>
    <AppearanceProvider>
    <HashRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route index element={<Suspense fallback={<Spinner />}><Dashboard /></Suspense>} />
          <Route path="/devices"  element={<Suspense fallback={<Spinner />}><Devices   /></Suspense>} />
          <Route path="/pairing"  element={<Suspense fallback={<Spinner />}><Pairing   /></Suspense>} />
          <Route path="/clipboard"element={<Suspense fallback={<Spinner />}><Clipboard /></Suspense>} />
          <Route path="/input"    element={<Suspense fallback={<Spinner />}><Input     /></Suspense>} />
          <Route path="/files"    element={<Suspense fallback={<Spinner />}><Files     /></Suspense>} />
          <Route path="/settings" element={<Suspense fallback={<Spinner />}><Settings  /></Suspense>} />
          <Route path="/logs"     element={<Suspense fallback={<Spinner />}><Logs      /></Suspense>} />
          <Route path="/about"    element={<Suspense fallback={<Spinner />}><About     /></Suspense>} />
        </Route>
      </Routes>
    </HashRouter>
    </AppearanceProvider>
    </DaemonProvider>
  </ErrorBoundary>
);

export default App;
