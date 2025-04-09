import { useParams } from "react-router-dom";
import { QuickBuildState } from "./QuickBuildState";
import { useQuery } from '@tanstack/react-query'

import { Layout } from "../../components/Layout";
import { maps } from '../../api/api'


export const QuickBuildStateView = () => {
    const {buildRequestId, repoId} = useParams();

    const { data } = useQuery(
        ['repo', repoId],
        () => maps.getRepo(repoId),
        { retry: 0, }
    )

    return <Layout scrollable legalFooter>
        <h1>Quick index: {data?.name}</h1>
        <QuickBuildState buildRequestId={buildRequestId} repoId={repoId} repoData={data} />
    </Layout>
}
