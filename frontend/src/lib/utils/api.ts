import axios from "./axios";
import { API_BASE_URL } from "./url";

export interface ApiResponse<T = any> {
  data: T;
  status: number;
  statusText: string;
}

export interface ApiError {
  code: number;
  error: string;
  message?: string;
}

class ApiClient {
  async get<T = any>(url: string, config?: any): Promise<T> {
    const response = await axios.get<T>(url, config);
    return response.data;
  }

  async post<T = any>(url: string, data?: any, config?: any): Promise<T> {
    const response = await axios.post<T>(url, data, config);
    return response.data;
  }

  async put<T = any>(url: string, data?: any, config?: any): Promise<T> {
    const response = await axios.put<T>(url, data, config);
    return response.data;
  }

  async patch<T = any>(url: string, data?: any, config?: any): Promise<T> {
    const response = await axios.patch<T>(url, data, config);
    return response.data;
  }

  async delete<T = any>(url: string, data?: any, config?: any): Promise<T> {
    // For DELETE requests with data, we need to put the data in the config
    const deleteConfig = data ? { ...config, data } : config;
    const response = await axios.delete<T>(url, deleteConfig);
    return response.data;
  }

  // For streaming responses that need direct fetch access
  async fetchStream(url: string, options: RequestInit = {}): Promise<Response> {
    const headers = new Headers(options.headers);

    // Add the frontend host header for authorization
    if (typeof window !== "undefined") {
      headers.set("X-Frontend-Host", window.location.host);
    }

    // Ensure credentials are included
    const fetchOptions: RequestInit = {
      ...options,
      credentials: "include",
      headers,
    };

    // Use absolute URL if needed
    const fullUrl = url.startsWith("/") ? `${API_BASE_URL}${url}` : url;

    return fetch(fullUrl, fetchOptions);
  }
}

export const api = new ApiClient();
export default api;
