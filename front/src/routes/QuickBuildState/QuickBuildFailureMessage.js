import { Link } from "react-router-dom";


export const QuickBuildFailureMessage = ({repoId}) => <div>
    It appears that we cannot automatically index this repo.
    Help us fill in the blanks by adding the repository <Link to={repoId ? `/repos/${repoId}/jobs` : "/repos/new"}>here</Link>.
</div>
