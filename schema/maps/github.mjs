const githubRegex = /(?:https?:\/\/github\.com\/|git@github\.com:|^)([^\/\s]+)\/([^\/\s\.]+)(?:\.git)?(?:\/)?$/;

export const decodeGithubUrl = (urlString) => {
    const match = urlString.match(githubRegex);
    if (match) {
        return {
            owner: match[1],
            repository: match[2]
        };
    }
    return null;
};
