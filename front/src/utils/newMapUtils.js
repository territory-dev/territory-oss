import { maps, createMap } from '../api/api'


export const repoMapDefaults = async (repoId) => {
    const repo = await maps.getRepo(repoId)
    const branch = repo.defaultBranch || (await maps.getBranches(repoId))[0].id
    const builds = await maps.getBuilds(repoId, branch)
    const buildId = builds[0].id
    const display_name = `Map of ${repo.name}`;

    return {repoId, branch, buildId, display_name}
}


export const createMapWithDefaults = async (repoId) => {
    const {repoId: _, branch, buildId, display_name} = await repoMapDefaults(repoId);

    return createMap({
        repoId,
        branchId: branch,
        buildId,
        display_name,
        public: false,
    })
}


export const createMapWithRetry = (mapOpts, created, attempts=10) => {
    console.log(mapOpts)
    if (attempts == 0) return;

    createMap(mapOpts).then(
        (mapId) => created(mapId),
        (e) => {
            console.error('createMap', e.response || e)
            setTimeout(
                () => createMapWithRetry(mapOpts, created, attempts-1),
                1000)
        })
}
