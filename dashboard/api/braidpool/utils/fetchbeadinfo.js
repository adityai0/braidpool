import axios from "axios";

const TIMEOUT = 10000;
const MAX_RETRIES = 2;

async function rpcCall(method, params = [], retries = MAX_RETRIES) {
  const url = process.env.BRAIDPOOL_NODE_URL;

  if (!url) {
    throw new Error("BRAIDPOOL_NODE_URL not set");
  }

  try {
    const response = await axios.post(
      url,
      {
        jsonrpc: "2.0",
        id: Date.now(),
        method,
        params,
      },
      {
        headers: { "Content-Type": "application/json" },
        timeout: TIMEOUT,
      }
    );

    if (response.data.error) {
      throw new Error(JSON.stringify(response.data.error));
    }

    return response.data.result;
  } catch (error) {
    if (retries > 0) {
      console.warn(`Retrying ${method}...`);
      return rpcCall(method, params, retries - 1);
    }
    console.error(`[RPC ERROR] ${method}:`, error.message);
    throw error;
  }
}
const getBeadCount = () => rpcCall("getbeadcount");
const getCohortCount = () => rpcCall("getcohortcount");
const getTips = () => rpcCall("gettips");
const getGenesis = () => rpcCall("getgenesis");
const getPeerInfo = () => rpcCall("getpeerinfo");
const getBraidInfo = () => rpcCall("getbraidinfo");
const getHighestWorkPathByCount = (count = 50) =>
  rpcCall("gethighestworkpathbycount", [count]);



export async function fetchBraidpoolBeadInfo() {
  const url = process.env.BRAIDPOOL_NODE_URL;

  if (!url) {
    console.warn("BRAIDPOOL_NODE_URL not set");
    return null;
  }

  console.log("Fetching Braidpool data from:", url);

  const results = await Promise.allSettled([
    getBeadCount(),
    getCohortCount(),
    getTips(),
    getGenesis(),
    getBraidInfo(),
    getPeerInfo(),
    getHighestWorkPathByCount(50),
  ]);

  const keys = [
    "beadCount",
    "cohortCount",
    "tips",
    "genesis",
    "braidInfo",
    "peerInfo",  
    "highestWorkPath",
  ];

  const data = {};
  const errors = [];

  results.forEach((result, i) => {
    if (result.status === "fulfilled") {
      data[keys[i]] = result.value;
    } else {
      data[keys[i]] = null;
      errors.push({
        method: keys[i],
        error: result.reason?.message,
      });
    }
  });

  if (errors.length > 0) {
    console.warn("Some RPC calls failed:", errors);
  }

  return data;
}