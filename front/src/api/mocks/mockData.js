import { index } from "./json"

export const mockData = {
    '/search': [
        { id: 'cur/18239200878938622542', text: '/'},
    ],
    '/projects': [{
        name: 'Linux kernel',
        id: 'linux',
        root: { id: 'cur/18239200878938622542', text: '/'},
    }],
    '/nodes/linux/index.json': index,
    '/maps': [
        {
            build: 'build1',
            id: 'ouY9KDKgp7XAQQlJFvr2',
            display_name: 'Map of linux',
            last_changes: 'October 6, 2023 at 7:01:24 PM UTC+2',
            owner: 'cblhzEfSWjRS4CHTD8bjMZAN7I63',
            public: false,
            graph: null,
        }
    ],
    '/builds/build1': {
        branch: 'main',
        build_ended: 'October 4, 2023 at 8:41:00 PM UTC+2',
        code_bucket: 'territory-index-prod',
        code_path: '/nodes/linux/pb/',
        code_root: 'cur/18239200878938622542',
        code_storage: 'firestore',
        commit: "deadbeef",
        commit_message: "commit text",
        indexer_version: "",
        public: true,
        repo: "linux",
        search_index_path: "/search/linux/all",
        status: "ready",
    },
}
