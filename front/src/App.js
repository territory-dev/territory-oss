import {
  createBrowserRouter,
  RouterProvider,
} from 'react-router-dom'
import {
  QueryClient,
  QueryClientProvider,
} from '@tanstack/react-query'
import { UserContextProvider } from './contexts/userContext'
import { MapsContextProvider } from './contexts/mapsContext'
import { routes } from './routes/routes'


import './App.css'

function App() {
  const router = createBrowserRouter(routes)
  const queryClient = new QueryClient()

  return (
    <UserContextProvider>
      <MapsContextProvider>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router}>
          </RouterProvider>
        </QueryClientProvider>
      </MapsContextProvider>
    </UserContextProvider>
  );
}

export default App;
