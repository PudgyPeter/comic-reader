import { Suspense, lazy, useEffect } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { Spinner } from "./components/ui";
import { useSettings } from "./store/settings";

const LibraryPage = lazy(() => import("./pages/LibraryPage"));
const SeriesPage = lazy(() => import("./pages/SeriesPage"));
const SeriesDetailPage = lazy(() => import("./pages/SeriesDetailPage"));
const ReaderPage = lazy(() => import("./pages/ReaderPage"));
const CollectionsPage = lazy(() => import("./pages/CollectionsPage"));
const CollectionDetailPage = lazy(() => import("./pages/CollectionDetailPage"));
const OrdersPage = lazy(() => import("./pages/OrdersPage"));
const OrderDetailPage = lazy(() => import("./pages/OrderDetailPage"));
const SearchPage = lazy(() => import("./pages/SearchPage"));
const StatsPage = lazy(() => import("./pages/StatsPage"));
const HistoryPage = lazy(() => import("./pages/HistoryPage"));
const SettingsPage = lazy(() => import("./pages/SettingsPage"));
const UnmatchedPage = lazy(() => import("./pages/UnmatchedPage"));
const CharactersPage = lazy(() => import("./pages/CharactersPage"));

function Themed() {
  const settings = useSettings((state) => state.settings);
  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme;
    document.documentElement.style.setProperty("--accent", settings.accent);
    document.documentElement.dataset.density = settings.density;
  }, [settings.theme, settings.accent, settings.density]);
  return null;
}

export default function App() {
  const load = useSettings((state) => state.load);
  const loaded = useSettings((state) => state.loaded);

  useEffect(() => {
    void load();
  }, [load]);

  if (!loaded) {
    return (
      <div className="grid h-full place-items-center">
        <Spinner className="size-6" />
      </div>
    );
  }

  return (
    <>
      <Themed />
      <Suspense
        fallback={
          <div className="grid h-full place-items-center">
            <Spinner className="size-6" />
          </div>
        }
      >
        <Routes>
          <Route path="reader/:issueId" element={<ReaderPage />} />
          <Route path="file/:fileId" element={<ReaderPage />} />
          <Route element={<AppShell />}>
            <Route index element={<LibraryPage />} />
            <Route path="series" element={<SeriesPage />} />
            <Route path="series/:seriesId" element={<SeriesDetailPage />} />
            <Route path="unmatched" element={<UnmatchedPage />} />
          <Route path="characters" element={<CharactersPage />} />
            <Route path="collections" element={<CollectionsPage />} />
            <Route path="collections/:collectionId" element={<CollectionDetailPage />} />
            <Route path="orders" element={<OrdersPage />} />
            <Route path="orders/:orderId" element={<OrderDetailPage />} />
            <Route path="search" element={<SearchPage />} />
            <Route path="stats" element={<StatsPage />} />
            <Route path="history" element={<HistoryPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </Suspense>
    </>
  );
}
