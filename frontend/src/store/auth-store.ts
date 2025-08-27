import { proxy } from "valtio";
import axios from "@/lib/axios";

export interface User {
  id: string;
  client_id: string;
  username: string;
  role: "user" | "admin" | "root";
}

export interface Client {
  id: string;
  name: string;
  description?: string;
  status?: "pending" | "installing" | "active" | "error";
  installPath?: string;
  createdAt?: string;
  updatedAt?: string;
}

interface AuthState {
  user: User | null;
  firstClient: Client | null;
  loading: boolean;
  isSetupComplete: boolean;
  needsInitialSetup: boolean;
  needsFirstUser: boolean;
  registrationEnabled: boolean;
  requireInviteCode: boolean;
}

// Initial auth state
const initialAuthState: AuthState = {
  user: null,
  firstClient: null,
  loading: true,
  isSetupComplete: false,
  needsInitialSetup: false,
  needsFirstUser: false,
  registrationEnabled: false,
  requireInviteCode: false,
};

// Create the auth store
export const authStore = proxy(initialAuthState);

// Computed property
export const isAuthenticated = () => !!authStore.user;

// Auth actions
export const setUser = (user: User | null) => {
  authStore.user = user;
};

export const setFirstClient = (client: Client | null) => {
  authStore.firstClient = client;
};

export const setLoading = (loading: boolean) => {
  authStore.loading = loading;
};

export const setSetupComplete = (complete: boolean) => {
  authStore.isSetupComplete = complete;
};

export const setNeedsInitialSetup = (needs: boolean) => {
  authStore.needsInitialSetup = needs;
};

export const setNeedsFirstUser = (needs: boolean) => {
  authStore.needsFirstUser = needs;
};

export const setRegistrationEnabled = (enabled: boolean) => {
  authStore.registrationEnabled = enabled;
};

export const setRequireInviteCode = (required: boolean) => {
  authStore.requireInviteCode = required;
};

// Auth functions
export const checkAuthStatus = async () => {
  try {
    const response = await axios.get("/auth/me");
    setUser(response.data.user);
    setSetupComplete(response.data.is_setup_complete);
    return response.data.user;
  } catch (error: any) {
    setUser(null);
    setSetupComplete(false);
    return null;
  }
};

export const fetchFirstClient = async () => {
  try {
    const response = await axios.get("/clients");
    if (response.data && response.data.length > 0) {
      const client = response.data[0];
      setFirstClient(client);
      if (client.id) {
        localStorage.setItem("activeClientId", client.id);
      }
      setNeedsInitialSetup(false);
      return client;
    } else {
      setFirstClient(null);
      setNeedsInitialSetup(true);
      return null;
    }
  } catch (error) {
    setNeedsInitialSetup(false);
    return null;
  }
};

export const checkUsersExist = async (clientId: string) => {
  try {
    const response = await axios.get("/auth/users/exists", {
      params: { client_id: clientId },
    });
    const usersExist = response.data.users_exist;
    setNeedsFirstUser(!usersExist);
  } catch (error) {
    setNeedsFirstUser(false);
  }
};

export const checkRegistrationStatus = async () => {

  if (!authStore.firstClient) {
    return;
  }

  try {
    const response = await axios.get("/auth/registration-status", {
      params: { client_id: authStore.firstClient.id },
    });
    setRegistrationEnabled(response.data.registration_enabled);
    setRequireInviteCode(response.data.require_invite_code);
  } catch (error) {
    console.error("Failed to fetch registration status:", error);
    setRegistrationEnabled(false);
    setRequireInviteCode(false);
  }
};

export const initializeApp = async () => {
  try {
    const clientData = await fetchFirstClient();
    const userData = await checkAuthStatus();

    // Check registration status after fetching the first client
    if (clientData) {
      await checkRegistrationStatus();
    }

    if (!userData && clientData?.status === "active" && clientData?.id) {
      await checkUsersExist(clientData.id);
    }
  } finally {
    setLoading(false);
  }
};

export const login = async (username: string, password: string) => {
  if (!authStore.firstClient) {
    throw new Error("No client available");
  }

  try {
    const response = await axios.post("/auth/login", {
      client_id: authStore.firstClient.id,
      username,
      password,
    });
    setUser(response.data.user);

    await new Promise((resolve) => setTimeout(resolve, 100));

    try {
      const meResponse = await axios.get("/auth/me");
      setSetupComplete(meResponse.data.is_setup_complete);
    } catch (error) {
      setSetupComplete(false);
    }
  } catch (error: any) {
    if (error.response?.data?.error) {
      throw new Error(error.response.data.error);
    }
    throw new Error("Login failed");
  }
};

export const register = async (
  username: string,
  password: string,
  inviteCode?: string
) => {
  if (!authStore.firstClient) {
    throw new Error("No client available");
  }

  try {
    const response = await axios.post("/auth/register", {
      client_id: authStore.firstClient.id,
      username,
      password,
      invite_code: inviteCode,
    });
    setUser(response.data.user);

    await new Promise((resolve) => setTimeout(resolve, 100));

    try {
      const meResponse = await axios.get("/auth/me");
      setSetupComplete(meResponse.data.is_setup_complete);
    } catch (error) {
      setSetupComplete(false);
    }
  } catch (error: any) {
    if (error.response?.data?.error) {
      throw new Error(error.response.data.error);
    }
    throw new Error("Registration failed");
  }
};

export const logout = async () => {
  try {
    await axios.post("/auth/logout");
  } catch (error) {
    // Logout error
  } finally {
    setUser(null);
    setSetupComplete(false);
  }
};
