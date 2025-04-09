import Joi from 'joi'


export const setDisplayNameSchema = Joi.object({
    displayName: Joi.string().trim().pattern(/^[a-z0-9-_]{1,30}$/i).required().messages({
        'string.empty': 'Repository name is required and must be a non-empty string',
        'any.required': 'Repository name is required',
        'string.pattern.base': 'Display name does not match the regex',
    }),
})


export const createRepoSchema = Joi.object({
    name: Joi.string().trim().pattern(/^[a-z0-9-_]{1,30}$/i).required().messages({
        'string.empty': 'Repository name is required and must be a non-empty string',
        'any.required': 'Repository name is required',
        'string.pattern.base': 'Repository name does not match the regex',
    }),
    public: Joi.boolean().valid(true).required().messages({
        'boolean.base': 'Public flag must be a boolean',
        'any.only': 'Only public repositories are currently allowed',
        'any.required': 'Public flag is required'
    }),
    manual: Joi.boolean().default(false),
    lang: Joi.string().valid('c', 'go', 'python').default('c'),
})

export const createTrackedRepoSchema = createRepoSchema.append({
    origin: Joi.string().uri({ scheme: ['https'] }).regex(RegExp('^https://github\.com/.*\.git$')).required().messages({
        'string.empty': 'Git origin is required',
        'string.uri': 'Git origin must be a valid https URL',
        'string.pattern.base': 'Only Github repositories are allowed at this time',
    }),
    tracked_branch: Joi.string().trim().required().messages({
        'string.empty': 'Tracked branch is required and must be a non-empty string',
        'any.required': 'Tracked branch is required'
    }),
    image: Joi.string().trim().required().messages({
        'string.empty': 'Docker image is required and must be a non-empty string',
        'any.required': 'Docker image is required'
    }),
    prepare_script: Joi.string().required().messages({
        'string.empty': 'Build script is required and must be a non-empty string',
        'any.required': 'Build script is required'
    }),
})


export const createUploadToken = Joi.object({
    display_name: Joi.string().max(512).default('unnamed token'),
})


export const repoIdStr = Joi.string().pattern(/^[^/]{1,100}$/)
export const branchStr = Joi.string().pattern(/^[^/]{1,100}$/)
export const encodedBranchStr = Joi.string().min(1).max(100)
export const buildIdStr = Joi.string().pattern(/^[^/]{1,100}$/)

export const createBuildRequest = Joi.object({
    repo_id: repoIdStr.required(),
    branch: encodedBranchStr.required(),
    len: Joi.number().required(),
    meta: Joi.object({
        commit: Joi.string().pattern(/^[^/]{1,100}$/).required(),
        commit_message: Joi.string(),
        repo_root: Joi.string().required(),
        compile_commands_dir: Joi.string(),
        index_system: Joi.bool().default(false),
        lang: Joi.string().valid('c', 'go', 'python').default('c'),
    }),
})


export const createMapRequest = Joi.object({
    buildId: buildIdStr.required(),
    branchId: encodedBranchStr.required(),
    repoId: repoIdStr.required(),
    display_name: Joi.string(),
    public: Joi.bool(),
    graph: Joi.object({
        nodes: Joi.object(),
        relations: Joi.object(),
        notes: Joi.object(),
    }),
})


export const fieldErrors = (error) => {
    const res = {}

    error.details.forEach(e => {
        res[e.context.key] = e.message
    })

    return {errors: res}
}
