import axios from 'axios'
import { API_BASE_URL } from './url'

// Configure axios defaults
axios.defaults.withCredentials = true
axios.defaults.baseURL = import.meta.env.VITE_API_URL || API_BASE_URL

// Add response interceptor to handle auth errors
axios.interceptors.response.use(
  (response) => response,
  (error) => {
    // Let the component handle 401 errors instead of auto-redirecting
    return Promise.reject(error)
  }
)

export default axios