import axios from "axios"

export interface GDPS {
    success: boolean;
    server: {
        srvid: string;
        srvName: string;
        description: string;
        icon: string;
        userCount: number;
        levelCount: number,
        textAlign: "left" | "center" | "right",
        backgroundImage: string
    }
}

export async function getGDPS(id: string) {
    const resp = await axios.get(`https://api.rigby.host/gdps/${id}/fetch`, {
        timeout: 300000,
    })
    return resp.data as GDPS
}